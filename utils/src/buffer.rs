pub struct Buffer<'a> {
  index: u8,
  data: &'a [u8],
}

impl <'a> Buffer<'a> {
  pub fn new(data: &'a [u8]) -> Option<Buffer<'a>> {
    if data.len() > (u8::MAX as usize) {
      return None
    }
    Some(Buffer { index: 0, data })
  }

  pub fn next_byte(&mut self) -> Option<u8> {
    let idx = self.index as usize;
    if idx >= self.data.len() {
      return None
    }
    let byte = self.data[idx];
    self.index += 1;
    Some(byte)
  }

  pub fn write(&mut self, dest: &mut [u8]) -> usize {
    self.write_with_size(dest, self.len())
  }

  pub fn write_with_size(&mut self, dest: &mut [u8], size: usize) -> usize {
    let mut index = 0;
    while !self.is_empty() && index < size {
      dest[index] = self.next_byte().unwrap();
      index += 1;
    }
    index
  }

  pub fn len(&self) -> usize {
    self.data.len() - (self.index as usize)
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn get_raw_buffer(&self) -> &[u8] {
    &self.data[(self.index as usize)..]
  }

  pub fn read_length(&mut self, length: usize) -> &[u8] {
    let to = (self.index as usize) + length;
    let result = &self.data[(self.index as usize)..to];
    self.index = to as u8;
    result
  }

  pub fn get_index(&self) -> u8 {
    self.index
  }

  pub fn get_range(&self, from_index: u8, to_index: u8) -> &[u8] {
    &self.data[(from_index as usize)..(to_index as usize)]
  }
}
