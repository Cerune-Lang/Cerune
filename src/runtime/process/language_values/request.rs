//! 起動前に要求の文字列・配列を独立したRust領域へ変換します。
use super::{Element, Error, IntegerType, Request, Value, definition, ir};
use std::time::Duration;

/// 本文をDebugや診断へ自動出力しません。要求はこの所有物の寿命内でのみ貸し出します。
pub(crate) struct PreparedRequest {
    target: String,
    arguments: Vec<String>,
    stdin: Vec<u8>,
    stdout: usize,
    stderr: usize,
    timeout: Duration,
}
impl PreparedRequest {
    pub(crate) fn new<V: Value>(
        definitions: &[ir::TypeDefinition],
        id: ir::TypeId,
        value: &V,
    ) -> Result<Self, Error> {
        Self::prepare(definitions, id, value, &mut || Ok(()))
    }
    pub(super) fn prepare<V: Value>(
        definitions: &[ir::TypeDefinition],
        id: ir::TypeId,
        value: &V,
        before_allocation: &mut impl FnMut() -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let def = definition(
            definitions,
            id,
            &[
                "target",
                "arguments",
                "stdin",
                "stdout_limit",
                "stderr_limit",
                "timeout_ms",
            ],
        )?;
        if def.fields[0].ty != ir::Type::String
            || !matches!(&def.fields[1].ty,ir::Type::DynamicArray{element} if **element==ir::Type::String)
            || !matches!(&def.fields[2].ty,ir::Type::DynamicArray{element} if **element==ir::Type::Integer(IntegerType::U8))
            || def.fields[3..]
                .iter()
                .any(|f| f.ty != ir::Type::Integer(IntegerType::U64))
        {
            return Err(Error::InvalidLayout);
        }
        let f = value
            .fields(id)
            .filter(|f| f.len() == 6)
            .ok_or(Error::InvalidValue)?;
        let target = f[0].text().ok_or(Error::InvalidValue)?.borrow_text();
        let args = f[1]
            .array(Element::String)
            .ok_or(Error::InvalidValue)?
            .borrow_values()?;
        let bytes = f[2]
            .array(Element::Byte)
            .ok_or(Error::InvalidValue)?
            .borrow_values()?;
        let number = |i: usize| f[i].unsigned(IntegerType::U64).ok_or(Error::InvalidValue);
        let stdout = usize::try_from(number(3)?).map_err(|_| Error::Size)?;
        let stderr = usize::try_from(number(4)?).map_err(|_| Error::Size)?;
        let timeout = Duration::from_millis(number(5)?);
        // 型が壊れた要求を途中までコピーせず、読取りだけで先に拒否します。
        if args.iter().any(|a| a.text().is_none())
            || bytes.iter().any(|b| b.unsigned(IntegerType::U8).is_none())
        {
            return Err(Error::InvalidValue);
        }
        let target = copy_text(&target, before_allocation)?;
        let mut arguments = Vec::new();
        before_allocation()?;
        arguments
            .try_reserve_exact(args.len())
            .map_err(|_| Error::Allocation)?;
        for a in args.iter() {
            arguments.push(copy_text(
                &a.text().unwrap().borrow_text(),
                before_allocation,
            )?);
        }
        let mut stdin = Vec::new();
        before_allocation()?;
        stdin
            .try_reserve_exact(bytes.len())
            .map_err(|_| Error::Allocation)?;
        for b in bytes.iter() {
            stdin.push(b.unsigned(IntegerType::U8).unwrap() as u8);
        }
        Ok(Self {
            target,
            arguments,
            stdin,
            stdout,
            stderr,
            timeout,
        })
    }
    /// 参照配列の確保もcallbackへ入る前です。実行後には確保しません。
    /// 値の妥当性・接続・要求上限の検査順は共通runtimeが引き続き決めます。
    pub(crate) fn with_request<R>(&self, run: impl FnOnce(Request<'_>) -> R) -> Result<R, Error> {
        let mut arguments = Vec::new();
        arguments
            .try_reserve_exact(self.arguments.len())
            .map_err(|_| Error::Allocation)?;
        arguments.extend(self.arguments.iter().map(String::as_str));
        Ok(run(Request {
            target: &self.target,
            arguments: &arguments,
            stdin: &self.stdin,
            stdout_limit: self.stdout,
            stderr_limit: self.stderr,
            timeout: self.timeout,
        }))
    }
}
fn copy_text(
    value: &str,
    before_allocation: &mut impl FnMut() -> Result<(), Error>,
) -> Result<String, Error> {
    before_allocation()?;
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| Error::Allocation)?;
    text.push_str(value);
    Ok(text)
}
