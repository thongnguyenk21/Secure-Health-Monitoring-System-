//! Numbers samples and keeps them until the gateway acknowledges them.

use iomt_frames::{Activity, Record};

use crate::ring::RingBuffer;

/// One second of measurements, before it gets a sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Device monotonic time in milliseconds since boot.
    pub mono_ms: u32,

    /// Heart rate in beats per minute; `0` means not available.
    pub hr_bpm: u16,

    /// Blood oxygen saturation in percent; `0` means not available.
    pub spo2_pct: u8,

    /// Steps counted during this second.
    pub steps: u16,

    /// Movement class.
    pub activity: Activity,

    /// Signal quality, 0 to 100.
    pub quality: u8,
}

/// Owns the offline buffer and the sequence numbering of one wearable.
///
/// The firmware calls [`record`](Self::record) every second, sends what
/// [`peek_batch`](Self::peek_batch) returns while the link is up, and calls
/// [`on_ack`](Self::on_ack) for each `LinkAck` from the gateway.
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

    /// Numbers a sample and stores it.
    ///
    /// # Arguments
    /// * `sample` - one second of measurements.
    ///
    /// # Returns
    /// The stored record, carrying its new sequence number.
    pub fn record(&mut self, sample: Sample) -> Record {
        let rec = Record {
            seq: self.next_seq,
            mono_ms: sample.mono_ms,
            hr_bpm: sample.hr_bpm,
            spo2_pct: sample.spo2_pct,
            steps: sample.steps,
            activity: sample.activity,
            quality: sample.quality,
        };
        self.next_seq = self.next_seq.wrapping_add(1);
        self.ring.push(rec);
        rec
    }

    /// Copies the oldest records into `out` without removing them.
    ///
    /// # Arguments
    /// * `out` - destination; at most `out.len()` records are copied.
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

    /// Copies records numbered after `after_seq` into `out`, oldest first, without removing them.
    ///
    /// The sender keeps a cursor (the last sequence number it wrote to the link)
    /// and calls this repeatedly with a small `out`, so it never needs a copy of
    /// the whole buffer, which matters on a microcontroller with a small stack.
    ///
    /// # Arguments
    /// * `after_seq` - skip every record whose sequence number is `<= after_seq`.
    /// * `out` - destination; at most `out.len()` records are copied.
    ///
    /// # Returns
    /// The number of records copied; `0` when everything was already sent.
    pub fn peek_after(&self, after_seq: u32, out: &mut [Record]) -> usize {
        let mut n = 0;
        let unsent = self.ring.iter().skip_while(|r| r.seq <= after_seq);
        for (slot, rec) in out.iter_mut().zip(unsent) {
            *slot = *rec;
            n += 1;
        }
        n
    }

    /// Handles a cumulative acknowledgement (`LinkAck.through_seq`).
    ///
    /// # Arguments
    /// * `through_seq` - every record up to and including this number was received.
    ///
    /// # Returns
    /// How many records were removed from the buffer.
    pub fn on_ack(&mut self, through_seq: u32) -> usize {
        self.ring.ack_through(through_seq)
    }

    /// Records waiting for an acknowledgement.
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

    fn sample(t: u32) -> Sample {
        Sample {
            mono_ms: t * 1000,
            hr_bpm: 72,
            spo2_pct: 98,
            steps: 1,
            activity: Activity::Walk,
            quality: 88,
        }
    }

    fn peek_seqs<const N: usize>(courier: &Courier<N>) -> Vec<u32> {
        let mut out = [Record::EMPTY; 16];
        let n = courier.peek_batch(&mut out);
        out[..n].iter().map(|r| r.seq).collect()
    }

    #[test]
    fn samples_are_numbered_from_one() {
        let mut courier: Courier<4> = Courier::new();
        assert_eq!(courier.record(sample(0)).seq, 1);
        assert_eq!(courier.record(sample(1)).seq, 2);
        assert_eq!(courier.pending(), 2);
    }

    #[test]
    fn peeking_does_not_remove_records() {
        let mut courier: Courier<4> = Courier::new();
        for t in 0..3 {
            courier.record(sample(t));
        }
        let mut out = [Record::EMPTY; 2];
        assert_eq!(courier.peek_batch(&mut out), 2);
        assert_eq!([out[0].seq, out[1].seq], [1, 2]);
        assert_eq!(courier.pending(), 3);
    }

    #[test]
    fn only_an_ack_removes_records() {
        let mut courier: Courier<4> = Courier::new();
        for t in 0..3 {
            courier.record(sample(t));
        }
        assert_eq!(courier.on_ack(2), 2);
        assert_eq!(peek_seqs(&courier), [3]);
    }

    #[test]
    fn a_dropped_link_resends_unacknowledged_records() {
        let mut courier: Courier<8> = Courier::new();
        for t in 0..3 {
            courier.record(sample(t));
        }
        assert_eq!(peek_seqs(&courier), [1, 2, 3], "sent, then the link drops");
        courier.record(sample(3));
        assert_eq!(
            peek_seqs(&courier),
            [1, 2, 3, 4],
            "everything is sent again"
        );
    }

    #[test]
    fn a_long_outage_loses_only_the_oldest_samples() {
        let mut courier: Courier<3> = Courier::new();
        for t in 0..5 {
            courier.record(sample(t));
        }
        assert_eq!(peek_seqs(&courier), [3, 4, 5]);
        assert_eq!((courier.pending(), courier.dropped()), (3, 2));
    }

    #[test]
    fn peek_after_skips_what_was_already_sent() {
        let mut courier: Courier<8> = Courier::new();
        for t in 0..5 {
            courier.record(sample(t));
        }
        let mut out = [Record::EMPTY; 8];
        let n = courier.peek_after(2, &mut out);
        assert_eq!(
            out[..n].iter().map(|r| r.seq).collect::<Vec<_>>(),
            [3, 4, 5]
        );
        assert_eq!(courier.peek_after(5, &mut out), 0, "nothing new to send");
        assert_eq!(courier.pending(), 5, "peeking never removes records");
    }

    #[test]
    fn peek_after_respects_the_batch_size() {
        let mut courier: Courier<8> = Courier::new();
        for t in 0..6 {
            courier.record(sample(t));
        }
        let mut out = [Record::EMPTY; 2];
        assert_eq!(courier.peek_after(0, &mut out), 2);
        assert_eq!([out[0].seq, out[1].seq], [1, 2]);
        assert_eq!(courier.peek_after(2, &mut out), 2);
        assert_eq!([out[0].seq, out[1].seq], [3, 4]);
    }

    #[test]
    fn peek_after_survives_overwritten_records() {
        let mut courier: Courier<3> = Courier::new();
        for t in 0..5 {
            courier.record(sample(t)); // seq 1 and 2 were overwritten
        }
        let mut out = [Record::EMPTY; 4];
        let n = courier.peek_after(1, &mut out);
        assert_eq!(
            out[..n].iter().map(|r| r.seq).collect::<Vec<_>>(),
            [3, 4, 5]
        );
    }
}
