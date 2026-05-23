pub struct StateReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> StateReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn skip(&mut self, len: usize) -> Option<()> {
        self.read_bytes(len).map(|_| ())
    }

    pub fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let bytes = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(bytes)
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        self.read_bytes(1).map(|bytes| bytes[0])
    }

    pub fn read_bool(&mut self) -> Option<bool> {
        self.read_u8().map(|value| value != 0)
    }

    pub fn read_u16_le(&mut self) -> Option<u16> {
        let bytes = self.read_bytes(2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u32_le(&mut self) -> Option<u32> {
        let bytes = self.read_bytes(4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_usize_le(&mut self) -> Option<usize> {
        let bytes = self.read_bytes(std::mem::size_of::<usize>())?;
        let mut value = [0u8; std::mem::size_of::<usize>()];
        value.copy_from_slice(bytes);
        Some(usize::from_le_bytes(value))
    }

    pub fn read_len_prefixed_u32(&mut self) -> Option<&'a [u8]> {
        let len = self.read_u32_le()? as usize;
        self.read_bytes(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_len_prefixed_u32_rejects_truncated_payload() {
        let mut reader = StateReader::new(&[4, 0, 0, 0, 1, 2]);

        assert!(reader.read_len_prefixed_u32().is_none());
    }

    #[test]
    fn read_bytes_rejects_overflowing_length() {
        let mut reader = StateReader::new(&[]);

        assert!(reader.read_bytes(usize::MAX).is_none());
    }
}
