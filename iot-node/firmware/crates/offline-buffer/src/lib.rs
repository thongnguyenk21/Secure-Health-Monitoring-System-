//! Keeps measurements safe while the wearable is out of range of the gateway.
//!
//! Design (decision D21): every sample is written into a fixed-size ring buffer
//! *first*. A sender takes copies of the oldest entries and transmits them; an
//! entry is removed only when the gateway acknowledges its sequence number.
//!
//! * Link up: the buffer is drained continuously and stays almost empty.
//! * Link down: it fills up; when full the OLDEST sample is overwritten and a
//!   counter remembers how many were lost.
//! * Link back: the backlog is sent oldest-first. If the link drops again
//!   mid-transfer nothing was removed yet, so nothing is lost (at-least-once).
#![cfg_attr(not(test), no_std)]

use iomt_frames::{Activity, Record};

/// Fixed-capacity FIFO of [`Record`]s that overwrites the oldest entry when full.
#[derive(Debug, Clone)]
pub struct RingBuffer<const N: usize> {
    buf: [Record; N],
    /// Index of the oldest entry.
    head: usize,
    len: usize,
    dropped: u32,
}

impl<const N: usize> RingBuffer<N> {
    /// Compile-time guard: a zero-capacity buffer is a programming error.
    const NON_EMPTY: () = assert!(N > 0, "RingBuffer capacity must be greater than zero");

    /// Creates an empty buffer holding up to `N` records.
    #[must_use]
    pub const fn new() -> Self {
        let () = Self::NON_EMPTY;
        Self {
            buf: [Record::EMPTY; N],
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    /// Appends a record, overwriting the oldest one if the buffer is full.
    ///
    /// # Returns
    /// `true` if an older record was overwritten (and counted as dropped).
    pub fn push(&mut self, rec: Record) -> bool {
        let overwrote = self.len == N;
        if overwrote {
            self.head = (self.head + 1) % N;
            self.len -= 1;
            self.dropped = self.dropped.saturating_add(1);
        }
        let tail = (self.head + self.len) % N;
        self.buf[tail] = rec;
        self.len += 1;
        overwrote
    }

    /// Returns the oldest record without removing it.
    #[must_use]
    pub fn oldest(&self) -> Option<&Record> {
        if self.len == 0 {
            None
        } else {
            Some(&self.buf[self.head])
        }
    }

    /// Removes and returns the oldest record.
    pub fn pop_oldest(&mut self) -> Option<Record> {
        let rec = *self.oldest()?;
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(rec)
    }

    /// Iterates from the oldest to the newest record.
    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        (0..self.len).map(move |i| &self.buf[(self.head + i) % N])
    }

    /// Number of records currently stored.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// `true` when no record is stored.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total number of records lost to overwriting since creation.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Removes every record whose sequence number is `<= seq`.
    ///
    /// # Returns
    /// How many records were removed.
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
    fn default() -> Self {
        Self::new()
    }
}

/// Numbers new samples and hands them to the link layer with ack-based removal.
#[derive(Debug, Clone)]
pub struct Courier<const N: usize> {
    ring: RingBuffer<N>,
    next_seq: u32,
}

impl<const N: usize> Courier<N> {
    /// Creates a courier with an empty buffer; the first sample gets seq 1.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ring: RingBuffer::new(),
            next_seq: 1,
        }
    }

    /// Stores a new measurement and returns it with its sequence number.
    ///
    /// # Arguments
    /// * `mono_ms` - device monotonic time in milliseconds.
    /// * `hr_bpm` - heart rate, `0` if unknown.
    /// * `spo2_pct` - blood oxygen, `0` if unknown.
    /// * `steps` - steps counted in this second.
    /// * `activity` - movement class.
    /// * `quality` - signal quality 0..=100.
    pub fn record_sample(
        &mut self,
        mono_ms: u32,
        hr_bpm: u16,
        spo2_pct: u8,
        steps: u16,
        activity: Activity,
        quality: u8,
    ) -> Record {
        let rec = Record {
            seq: self.next_seq,
            mono_ms,
            hr_bpm,
            spo2_pct,
            steps,
            activity,
            quality,
        };
        self.next_seq = self.next_seq.wrapping_add(1);
        self.ring.push(rec);
        rec
    }

    /// Copies up to `out.len()` of the oldest records into `out` WITHOUT removing them.
    ///
    /// # Returns
    /// The number of records copied.
    pub fn peek_batch(&self, out: &mut [Record]) -> usize {
        let mut n = 0;
        for (slot, rec) in out.iter_mut().zip(self.ring.iter()) {
            *slot = *rec;
            n += 1;
        }
        n
    }

    /// Handles a cumulative acknowledgement from the gateway.
    ///
    /// # Returns
    /// How many records were removed from the buffer.
    pub fn on_ack(&mut self, seq: u32) -> usize {
        self.ring.ack_through(seq)
    }

    /// Records waiting to be acknowledged.
    #[must_use]
    pub const fn pending(&self) -> usize {
        self.ring.len()
    }

    /// Records lost because the buffer overflowed while offline.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.ring.dropped()
    }
}

impl<const N: usize> Default for Courier<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add<const N: usize>(c: &mut Courier<N>, t: u32) -> Record {
        c.record_sample(t, 70, 98, 1, Activity::Walk, 90)
    }

    fn seqs(batch: &[Record], n: usize) -> Vec<u32> {
        batch[..n].iter().map(|r| r.seq).collect()
    }

    #[test]
    fn ring_overwrites_the_oldest_when_full() {
        let mut ring = RingBuffer::<3>::new();
        for seq in 1..=4 {
            ring.push(Record {
                seq,
                ..Record::EMPTY
            });
        }
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.dropped(), 1);
        assert_eq!(ring.iter().map(|r| r.seq).collect::<Vec<_>>(), [2, 3, 4]);
    }

    #[test]
    fn online_flow_keeps_the_buffer_nearly_empty() {
        let mut c = Courier::<8>::new();
        let mut batch = [Record::EMPTY; 4];
        for t in 0..10 {
            let rec = add(&mut c, t * 1000);
            let n = c.peek_batch(&mut batch);
            assert_eq!(n, 1);
            assert_eq!(batch[0], rec);
            c.on_ack(rec.seq);
            assert_eq!(c.pending(), 0);
        }
        assert_eq!(c.dropped(), 0);
    }

    #[test]
    fn offline_then_drain_in_order() {
        let mut c = Courier::<16>::new();
        for t in 0..5 {
            add(&mut c, t * 1000);
        }
        assert_eq!(c.pending(), 5);

        let mut batch = [Record::EMPTY; 3];
        let n = c.peek_batch(&mut batch);
        assert_eq!(seqs(&batch, n), [1, 2, 3]);
        assert_eq!(c.on_ack(3), 3);

        let n = c.peek_batch(&mut batch);
        assert_eq!(seqs(&batch, n), [4, 5]);
        assert_eq!(c.on_ack(5), 2);
        assert_eq!(c.pending(), 0);
    }

    #[test]
    fn link_drop_without_ack_loses_nothing() {
        let mut c = Courier::<8>::new();
        for t in 0..3 {
            add(&mut c, t);
        }
        let mut first = [Record::EMPTY; 3];
        let mut second = [Record::EMPTY; 3];
        c.peek_batch(&mut first); // sent, but the link dropped before any ack
        c.peek_batch(&mut second); // after reconnecting we send again
        assert_eq!(first, second);
        assert_eq!(c.pending(), 3);
    }

    #[test]
    fn overflow_while_offline_drops_the_oldest() {
        let mut c = Courier::<4>::new();
        for t in 0..6 {
            add(&mut c, t);
        }
        assert_eq!(c.pending(), 4);
        assert_eq!(c.dropped(), 2);
        let mut batch = [Record::EMPTY; 4];
        let n = c.peek_batch(&mut batch);
        assert_eq!(seqs(&batch, n), [3, 4, 5, 6]);
    }

    #[test]
    fn acks_are_cumulative_and_harmless_when_stale() {
        let mut c = Courier::<8>::new();
        for t in 0..4 {
            add(&mut c, t);
        }
        assert_eq!(c.on_ack(2), 2);
        assert_eq!(c.on_ack(1), 0); // old ack arrives late: nothing to remove
        assert_eq!(c.on_ack(99), 2); // ack beyond the end clears everything
        assert_eq!(c.pending(), 0);
    }
}
