use near_sdk::{env, near, require, Timestamp};

const NANOS_IN_SEC: u64 = 1_000_000_000;

/// Defines the delays in seconds for all critical stages of a swap, relative to its creation time.
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct TimelockDelays {
    // --- Source Chain Delays (e.g., NEAR -> Other) ---
    pub src_withdrawal_delay: u64,
    pub src_public_withdrawal_delay: u64,
    pub src_cancellation_delay: u64,
    pub src_public_cancellation_delay: u64,

    // --- Destination Chain Delays (e.g., Other -> NEAR) ---
    pub dst_withdrawal_delay: u64,
    pub dst_public_withdrawal_delay: u64,
    pub dst_cancellation_delay: u64,
}

/// A runtime object that combines creation time with delay configuration to manage swap stages.
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Timelocks {
    pub created_at: Timestamp,
    pub delays: TimelockDelays,
}

impl Timelocks {
    pub fn new(created_at: Timestamp, delays: TimelockDelays) -> Self {
        Self { created_at, delays }
    }

    // --- HELPER METHODS ---

    /// Asserts the current time is valid for a `withdrawal` (claim) on the destination chain.
    pub fn assert_dst_withdrawal_window(&self, is_public_caller: bool) {
        let now = env::block_timestamp();

        if is_public_caller {
            let public_withdrawal_start =
                self.created_at + self.delays.dst_public_withdrawal_delay * NANOS_IN_SEC;
            require!(
                now >= public_withdrawal_start,
                "Public withdrawal period (dst) has not started"
            );
        } else {
            let withdrawal_start =
                self.created_at + self.delays.dst_withdrawal_delay * NANOS_IN_SEC;
            require!(
                now >= withdrawal_start,
                "Private withdrawal period (dst) has not started"
            );
        }
        let cancellation_start =
            self.created_at + self.delays.dst_cancellation_delay * NANOS_IN_SEC;
        require!(
            now < cancellation_start,
            "Cancellation period (dst) has started"
        );
    }

    /// Asserts the current time is valid for a `withdrawal` (claim) on the source chain.
    pub fn assert_src_withdrawal_window(&self, is_public_caller: bool) {
        let now = env::block_timestamp();

        if is_public_caller {
            let public_withdrawal_start =
                self.created_at + self.delays.src_public_withdrawal_delay * NANOS_IN_SEC;
            require!(
                now >= public_withdrawal_start,
                "Public withdrawal period (src) has not started"
            );
        } else {
            let withdrawal_start =
                self.created_at + self.delays.src_withdrawal_delay * NANOS_IN_SEC;
            require!(
                now >= withdrawal_start,
                "Private withdrawal period (src) has not started"
            );
        }
        let cancellation_start =
            self.created_at + self.delays.src_cancellation_delay * NANOS_IN_SEC;
        require!(
            now < cancellation_start,
            "Cancellation period (src) has started"
        );
    }

    /// Asserts the current time is valid for a `cancellation` (refund) on the destination chain.
    pub fn assert_dst_cancellation_window(&self) {
        let now = env::block_timestamp();
        let cancellation_start =
            self.created_at + self.delays.dst_cancellation_delay * NANOS_IN_SEC;
        require!(
            now >= cancellation_start,
            "Cancellation period (dst) has not started"
        );
    }

    /// Asserts the current time is valid for a `cancellation` (refund) on the source chain.
    pub fn assert_src_cancellation_window(&self, is_public_caller: bool) {
        let now = env::block_timestamp();

        if is_public_caller {
            let public_cancellation_start =
                self.created_at + self.delays.src_public_cancellation_delay * NANOS_IN_SEC;
            require!(
                now >= public_cancellation_start,
                "Public cancellation period (src) has not started"
            );
        } else {
            let cancellation_start =
                self.created_at + self.delays.src_cancellation_delay * NANOS_IN_SEC;
            require!(
                now >= cancellation_start,
                "Private cancellation period (src) has not started"
            );
        }
    }
}

impl TimelockDelays {
    /// Validates the internal consistency of the delay settings.
    /// This prevents the creation of swaps with illogical time windows.
    /// It must be called before an escrow is created.
    pub fn validate(&self) {
        // --- Source Chain Validation ---
        // The private withdrawal period must start before the public one.
        require!(
            self.src_withdrawal_delay <= self.src_public_withdrawal_delay,
            "SRC: Public withdrawal cannot start before private"
        );
        // The withdrawal periods must start before the cancellation period.
        require!(
            self.src_public_withdrawal_delay < self.src_cancellation_delay,
            "SRC: Cancellation cannot start before public withdrawal ends"
        );
        // The private cancellation period must start before or at the same time as the public one.
        require!(
            self.src_cancellation_delay <= self.src_public_cancellation_delay,
            "SRC: Public cancellation cannot start before private"
        );

        // --- Destination Chain Validation ---
        // The private withdrawal period must start before the public one.
        require!(
            self.dst_withdrawal_delay <= self.dst_public_withdrawal_delay,
            "DST: Public withdrawal cannot start before private"
        );
        // The withdrawal periods must start before the cancellation period.
        require!(
            self.dst_public_withdrawal_delay < self.dst_cancellation_delay,
            "DST: Cancellation cannot start before public withdrawal ends"
        );

        // --- Cross-Chain Sanity Check ---
        // A destination cancellation should not happen after a source cancellation is possible.
        // This prevents a scenario where the resolver is stuck.
        require!(
            self.dst_cancellation_delay <= self.src_cancellation_delay,
            "X-CHAIN: Destination cancellation must not be after source cancellation"
        );
    }
}
