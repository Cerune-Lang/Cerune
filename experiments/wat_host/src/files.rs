//! The seven file imports adapt byte transport only, not Cerune file semantics.
use cerune_lang::runtime::files::{Backend, Handle, OpenError, Operation};
use std::{collections::BTreeMap, io};

#[derive(Debug)]
pub struct ProtocolError;
impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid file host protocol")
    }
}
impl std::error::Error for ProtocolError {}

struct Pending {
    bytes: [u8; 255],
    length: usize,
    offset: usize,
}
pub struct Files<B: Backend> {
    backend: Option<B>,
    pending: Option<Pending>,
    handles: BTreeMap<i32, (Operation, B::Handle)>,
    next: i64,
    stopped: bool,
}
impl<B: Backend> Files<B> {
    pub fn new(backend: Option<B>) -> Self {
        Self {
            backend,
            pending: None,
            handles: BTreeMap::new(),
            next: 1,
            stopped: false,
        }
    }
    fn active(&self) -> Result<(), ProtocolError> {
        if self.stopped {
            Err(ProtocolError)
        } else {
            Ok(())
        }
    }
    pub fn status(&self) -> Result<i32, ProtocolError> {
        self.active()?;
        Ok(i32::from(self.backend.is_none()))
    }
    pub fn begin(&mut self, length: i32) -> Result<i32, ProtocolError> {
        self.active()?;
        self.pending = None;
        if self.backend.is_none() || !(1..=255).contains(&length) {
            return Err(ProtocolError);
        }
        self.pending = Some(Pending {
            bytes: [0; 255],
            length: length as usize,
            offset: 0,
        });
        Ok(0)
    }
    pub fn chunk(&mut self, bits: i64, count: i32) -> Result<i32, ProtocolError> {
        self.active()?;
        let count = count_bytes(count)?;
        let pending = self.pending.as_mut().ok_or(ProtocolError)?;
        if pending.offset + count > pending.length {
            self.pending = None;
            return Err(ProtocolError);
        }
        pending.bytes[pending.offset..pending.offset + count]
            .copy_from_slice(&bits.to_le_bytes()[..count]);
        pending.offset += count;
        Ok(0)
    }
    pub fn open(&mut self, mode: i32) -> Result<(i32, i32), ProtocolError> {
        self.active()?;
        let pending = self.pending.take().ok_or(ProtocolError)?;
        let operation = match mode {
            0 => Operation::ReadText,
            1 => Operation::WriteNew,
            _ => return Err(ProtocolError),
        };
        if pending.length != pending.offset {
            return Err(ProtocolError);
        }
        let name =
            std::str::from_utf8(&pending.bytes[..pending.length]).map_err(|_| ProtocolError)?;
        let backend = self.backend.as_mut().ok_or(ProtocolError)?;
        if self.next > i64::from(i32::MAX) {
            return Ok((0, 4));
        }
        match backend.open(name, operation) {
            Ok(handle) => {
                let id = self.next as i32;
                self.next += 1;
                self.handles.insert(id, (operation, handle));
                Ok((id, 0))
            }
            Err(error) => Ok((
                0,
                match error {
                    OpenError::NotFound => 1,
                    OpenError::AlreadyExists => 2,
                    OpenError::UnsupportedTarget => 3,
                    OpenError::Failed => 4,
                    OpenError::InvalidResponse => 5,
                },
            )),
        }
    }
    pub fn read(&mut self, id: i32, requested: i32) -> Result<(i64, i32), ProtocolError> {
        self.active()?;
        let count = count_bytes(requested)?;
        let (operation, handle) = self.handles.get_mut(&id).ok_or(ProtocolError)?;
        if *operation != Operation::ReadText {
            return Err(ProtocolError);
        }
        let mut bytes = [0; 8];
        use std::io::Read;
        let accepted = response(handle.read(&mut bytes[..count]), count);
        Ok((
            if accepted > 0 && accepted <= requested {
                i64::from_le_bytes(bytes)
            } else {
                0
            },
            accepted,
        ))
    }
    pub fn write(&mut self, id: i32, bits: i64, requested: i32) -> Result<i32, ProtocolError> {
        self.active()?;
        let count = count_bytes(requested)?;
        let (operation, handle) = self.handles.get_mut(&id).ok_or(ProtocolError)?;
        if *operation != Operation::WriteNew {
            return Err(ProtocolError);
        }
        use std::io::Write;
        Ok(response(handle.write(&bits.to_le_bytes()[..count]), count))
    }
    pub fn close(&mut self, id: i32) -> Result<i32, ProtocolError> {
        self.active()?;
        let (_, handle) = self.handles.remove(&id).ok_or(ProtocolError)?;
        Ok(if handle.close().is_ok() { 0 } else { -1 })
    }
    pub fn shutdown(&mut self) {
        self.stopped = true;
        self.pending = None;
        while let Some((_, (_, handle))) = self.handles.pop_first() {
            let _ = handle.close();
        }
        self.backend = None;
    }
}
impl<B: Backend> Drop for Files<B> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
pub fn count_bytes(count: i32) -> Result<usize, ProtocolError> {
    if (1..=8).contains(&count) {
        Ok(count as usize)
    } else {
        Err(ProtocolError)
    }
}
pub fn response(result: io::Result<usize>, requested: usize) -> i32 {
    match result {
        Ok(n) if n <= requested => n as i32,
        Ok(_) => requested as i32 + 1,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => -2,
        Err(_) => -1,
    }
}
