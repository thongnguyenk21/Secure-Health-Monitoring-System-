//! Fixed-capacity queue of records that overwrites the oldest one when full.

use iomt_frames::Record;

/// Ring buffer of up to `N` records, oldest first.
///
/// Storage is a plain array, so the size is fixed at compile time and nothing
/// is allocated. `head` is the index of the oldest record; the others follow
/// it, wrapping around the end of the array.
#[derive(Debug, Clone)]
pub struct RingBuffer<const N: usize> {
    slots: [Record; N],
    head: usize,
    len: usize,
    dropped: u32,
}

impl<const N: usize> RingBuffer<N> {
    /// Creates an empty buffer.
    #[must_use]
    pub const fn new() -> Self {
        const {
            assert!(N > 0, "RingBuffer cap must be greater than 0");
        }
        Self {
            slots: [Record::EMPTY; N],
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    /// Maximum number of records the buffer holds.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Number of records stored.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// `true` when no record is stored.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Records lost to overwriting since the buffer was created.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Adds a record after the newest one.
    ///
    /// # Arguments
    /// * `rec` - the record to store.
    ///
    /// # Returns
    /// `true` if the buffer was full and the oldest record was overwritten (and
    /// counted in [`dropped`](Self::dropped)), `false` if there was room.
    pub fn push(&mut self, rec: Record) -> bool {
        if self.len == N {
            self.slots[self.head] = rec;
            self.head = (self.head + 1) % N;
            self.dropped = self.dropped.saturating_add(1);
            true
        } else {
            self.slots[(self.head + self.len) % N] = rec;
            self.len += 1;
            false
        }
    }

    /// The oldest record, if any.
    #[must_use]
    pub fn oldest(&self) -> Option<&Record> {
        if self.len > 0 {
            Some(&self.slots[self.head])
        } else {
            None
        }
    }

    /// Removes and returns the oldest record, or `None` when the buffer is empty.
    pub fn pop_oldest(&mut self) -> Option<Record> {
        if self.len == 0 {
            return None;
        }
        let rec = self.slots[self.head];
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(rec)
    }

    /// Iterates over the records from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        (0..self.len).map(move |i| &self.slots[(self.head + i) % N])
    }

    /// Removes every record whose sequence number is `<= seq`.
    ///
    /// # Arguments
    /// * `seq` - the highest sequence number the receiver has acknowledged.
    ///
    /// # Returns
    /// How many records were removed; `0` if `seq` is older than the oldest record.
    pub fn ack_through(&mut self, seq: u32) -> usize {
        let mut removed = 0;
        while self.oldest().is_some_and(|r| r.seq <= seq) {
            self.pop_oldest();
            removed += 1;
        }
        removed
    }
}

impl<const N: usize> Default for RingBuffer<N> {
    /// Creates an empty buffer.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iomt_frames::Activity;

    fn rec(seq: u32) -> Record {
        Record {
            seq,
            mono_ms: seq * 1000,
            hr_bpm: 70,
            spo2_pct: 98,
            steps: 0,
            activity: Activity::Rest,
            quality: 90,
        }
    }

    fn seqs<const N: usize>(ring: &RingBuffer<N>) -> Vec<u32> {
        ring.iter().map(|r| r.seq).collect()
    }

    #[test]
    fn a_new_buffer_is_empty() {
        let ring: RingBuffer<3> = RingBuffer::new();
        assert!(ring.is_empty());
        assert_eq!(ring.oldest(), None);
        assert_eq!(ring.capacity(), 3);
    }

    #[test]
    fn records_come_out_oldest_first() {
        let mut ring: RingBuffer<3> = RingBuffer::new();
        for seq in 1..=3 {
            ring.push(rec(seq));
        }
        assert_eq!(seqs(&ring), [1, 2, 3]);
        assert_eq!(ring.pop_oldest().map(|r| r.seq), Some(1));
        assert_eq!(seqs(&ring), [2, 3]);
    }

    #[test]
    fn a_full_buffer_overwrites_the_oldest_and_counts_it() {
        let mut ring: RingBuffer<3> = RingBuffer::new();
        for seq in 1..=3 {
            assert!(!ring.push(rec(seq)));
        }
        assert!(ring.push(rec(4)));
        assert_eq!(seqs(&ring), [2, 3, 4]);
        assert_eq!((ring.len(), ring.dropped()), (3, 1));
    }

    #[test]
    fn order_survives_wrapping_around_the_array() {
        let mut ring: RingBuffer<3> = RingBuffer::new();
        ring.push(rec(1));
        ring.push(rec(2));
        ring.pop_oldest();
        ring.push(rec(3));
        ring.push(rec(4));
        assert_eq!(seqs(&ring), [2, 3, 4]);
        ring.push(rec(5));
        assert_eq!(seqs(&ring), [3, 4, 5]);
    }

    #[test]
    fn ack_removes_everything_up_to_the_sequence() {
        let mut ring: RingBuffer<8> = RingBuffer::new();
        for seq in 1..=5 {
            ring.push(rec(seq));
        }
        assert_eq!(ring.ack_through(3), 3);
        assert_eq!(seqs(&ring), [4, 5]);
        assert_eq!(ring.ack_through(3), 0, "a repeated ack changes nothing");
        assert_eq!(ring.ack_through(10), 2);
        assert!(ring.is_empty());
    }

    #[test]
    fn an_ack_older_than_the_buffer_removes_nothing() {
        let mut ring: RingBuffer<3> = RingBuffer::new();
        for seq in 1..=5 {
            ring.push(rec(seq));
        }
        assert_eq!(ring.ack_through(2), 0);
        assert_eq!(seqs(&ring), [3, 4, 5]);
    }
}
