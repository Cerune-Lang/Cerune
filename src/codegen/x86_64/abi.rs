use super::Target;
use super::ir::ArgumentLocation;

/// 呼び出し側と受け取り側で、同じ順序・同じ規則で引数の場所を決めます。
pub(super) struct Arguments {
    target: Target,
    position: usize,
    integers: usize,
    floats: usize,
    stack: usize,
}

impl Arguments {
    pub(super) fn new(target: Target) -> Self {
        Self {
            target,
            position: 0,
            integers: 0,
            floats: 0,
            stack: 0,
        }
    }

    pub(super) fn next(&mut self, float: bool) -> ArgumentLocation {
        let register = if self.target.is_linux() {
            let (used, limit) = if float {
                (&mut self.floats, 8)
            } else {
                (&mut self.integers, 6)
            };
            let index = *used;
            *used += 1;
            (index < limit).then_some(index)
        } else {
            (self.position < 4).then_some(self.position)
        };
        self.position += 1;
        if let Some(index) = register {
            ArgumentLocation::Register(index)
        } else {
            let offset = self.stack_bytes();
            self.stack += 1;
            ArgumentLocation::Stack(offset)
        }
    }

    /// Windowsのshadow spaceを含む、呼び出し時のRSPからの必要バイト数です。
    pub(super) fn stack_bytes(&self) -> usize {
        (if self.target.is_linux() { 0 } else { 32 }) + 8 * self.stack
    }
}

pub(super) fn integer_argument_register(index: usize, target: super::Target) -> u8 {
    if target.is_linux() {
        [7, 6, 2, 1, 8, 9][index]
    } else {
        [1, 2, 8, 9][index]
    }
}
