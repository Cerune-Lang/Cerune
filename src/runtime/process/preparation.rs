//! 起動前の状態を所有し、消費したときだけ共通runtimeを進めます。
use super::*;

/// 不正要求等は結果が確定し、予約可能な要求だけがPendingになります。
/// 大きい方をBox化すると、従来なかった失敗可能な確保が必要になります。
#[allow(clippy::large_enum_variant)]
#[must_use]
pub enum Preparation<'p, 'r, B: Backend> {
    Complete(Outcome),
    Pending(Prepared<'p, 'r, B>),
}

/// 借用中は要求・接続を変更できず、executeはselfを消費します。
/// Dropは未提出の転送bufferだけを解放し、起動・時計・後始末を実行しません。
#[must_use]
pub struct Prepared<'p, 'r, B: Backend> {
    pub(super) processes: &'p mut Processes<B>,
    pub(super) request: Request<'r>,
    pub(super) outcome: Outcome,
    pub(super) transfers: Transfers,
}
impl<B: Backend> Prepared<'_, '_, B> {
    /// 接続も準備中は再利用できません。
    /// ```compile_fail
    /// use cerune_lang::runtime::process::{Backend, Processes, Request, Origin, Preparation};
    /// fn busy<B: Backend>(p: &mut Processes<B>, request: Request<'_>, origin: Origin) {
    ///     if let Preparation::Pending(prepared) = p.prepare(request, 32768, origin) {
    ///         p.run(request, 32768, origin);
    ///         prepared.execute(Ok(()));
    ///     }
    /// }
    /// ```
    /// 予約前の記録を読み取ります。実行制御・時計取得は行いません。
    pub fn report(&self) -> &Report {
        &self.outcome.report
    }

    /// 生成側の結果領域予約の結果を渡し、成功時だけ起動します。
    /// 成功した予約の所有物は、起動の成否にかかわらず返却します。
    ///
    /// 消費した準備状態は再利用できません。
    /// ```compile_fail
    /// use cerune_lang::runtime::process::{Backend, Prepared};
    /// fn twice<B: Backend>(prepared: Prepared<'_, '_, B>) {
    ///     prepared.execute(Ok(()));
    ///     prepared.execute(Ok(()));
    /// }
    /// ```
    pub fn execute<R>(self, reservation: Result<R, ReservationError>) -> (Outcome, Option<R>) {
        let Self {
            processes,
            request,
            mut outcome,
            transfers,
        } = self;
        let storage = match reservation {
            Ok(storage) => {
                if let Err(error) = processes.execute_prepared(&request, transfers, &mut outcome) {
                    outcome.report.failure.get_or_insert(error);
                }
                Some(storage)
            }
            Err(error) => {
                drop(transfers);
                outcome.report.failure = Some(failure(
                    match error {
                        ReservationError::Capacity => Code::OutputBudget,
                        ReservationError::Allocation => Code::AllocationFailed,
                    },
                    Phase::Reserve,
                ));
                None
            }
        };
        processes.record_finished(&outcome);
        (outcome, storage)
    }
}
