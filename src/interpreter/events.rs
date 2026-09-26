use super::*;

impl Interpreter {
    /// ADR-0052: Emit an event to the event stream.
    /// Thread-safe: appends to event_log behind Mutex, auto-increments ID.
    pub(super) fn emit_event(
        &self,
        event_type: &str,
        source: &str,
        data: HashMap<String, String>,
        duration_ms: Option<u64>,
    ) {
        let id = self
            .event_next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let event = Event {
            id,
            timestamp,
            event_type: event_type.to_string(),
            source: source.to_string(),
            data,
            duration_ms,
        };
        if let Ok(mut log) = self.event_log.lock() {
            log.push(event);
        }
    }

    /// ADR-0052: Get total number of events, optionally filtered by type.
    /// №466 group 5: the body lives in the shared live module
    /// (src/audit_ops.rs); this public test/contract wrapper delegates.
    pub fn event_count(&self, event_type: Option<&str>) -> usize {
        crate::audit_ops::event_count_in(&self.event_log, event_type)
    }

    /// ADR-0052: Get events since a given Unix timestamp (seconds).
    /// Returns events with timestamp >= since_ms (milliseconds).
    /// №466 group 5: delegates to the shared live module (src/audit_ops.rs).
    pub fn events_since_ms(&self, since_ms: u64) -> Vec<Event> {
        crate::audit_ops::events_since_in(&self.event_log, since_ms)
    }

    /// ADR-0052: Get a reference to the full event log (for test access).
    pub fn get_events(&self) -> Vec<Event> {
        self.event_log
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }

    /// ADR-0052: Sum a numeric field across events of a given type.
    /// Parses field values as f64 and sums them.
    /// №466 group 5: delegates to the shared live module (src/audit_ops.rs).
    pub fn event_sum(&self, event_type: &str, field: &str) -> f64 {
        crate::audit_ops::event_sum_in(&self.event_log, event_type, field)
    }
}
