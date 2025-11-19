pub struct RingBuffer {
    data: Vec<i16>,
    index: usize,
    filled: bool,
}

impl RingBuffer {
    pub fn new(size: usize) -> Self {
        RingBuffer {
            data: vec![0; size.max(1)],
            index: 0,
            filled: false,
        }
    }

    pub fn push(&mut self, sample: i16) {
        if self.data.is_empty() {
            return;
        }
        self.data[self.index] = sample;
        self.index = (self.index + 1) % self.data.len();
        if self.index == 0 {
            self.filled = true;
        }
    }
}
