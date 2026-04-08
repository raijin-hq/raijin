pub use tracing::{Level, field};

#[cfg(raijin_tracing)]
pub use tracing::{
    Span, debug_span, error_span, event, info_span, instrument, span, trace_span, warn_span,
};

#[cfg(not(raijin_tracing))]
pub use raijin_tracing_macro::instrument;

#[cfg(raijin_tracing)]
const MAX_CALLSTACK_DEPTH: u16 = 16;

#[cfg(all(raijin_tracing, raijin_tracing_with_memory))]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, MAX_CALLSTACK_DEPTH);

#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as trace_span;
#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as info_span;
#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as debug_span;
#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as warn_span;
#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as error_span;
#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as event;
#[cfg(not(raijin_tracing))]
pub use __consume_all_tokens as span;

#[cfg(not(raijin_tracing))]
#[macro_export]
macro_rules! __consume_all_tokens {
    ($($t:tt)*) => {
        $crate::Span
    };
}

#[cfg(not(raijin_tracing))]
pub struct Span;

#[cfg(not(raijin_tracing))]
impl Span {
    pub fn current() -> Self {
        Self
    }

    pub fn enter(&self) {}

    pub fn record<T, S>(&self, _t: T, _s: S) {}
}

#[cfg(raijin_tracing)]
pub fn init() {
    use tracing_subscriber::fmt::format::DefaultFields;
    use tracing_subscriber::prelude::*;

    #[derive(Default)]
    struct TracyLayerConfig {
        fmt: DefaultFields,
    }

    impl tracing_tracy::Config for TracyLayerConfig {
        type Formatter = DefaultFields;

        fn formatter(&self) -> &Self::Formatter {
            &self.fmt
        }

        fn stack_depth(&self, _: &tracing::Metadata) -> u16 {
            MAX_CALLSTACK_DEPTH
        }

        fn format_fields_in_zone_name(&self) -> bool {
            true
        }

        fn on_error(&self, client: &tracy_client::Client, error: &'static str) {
            client.color_message(error, 0xFF000000, 0);
        }
    }

    raijin_log::info!("Starting tracy subscriber, you can now connect the profiler");
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_tracy::TracyLayer::new(TracyLayerConfig::default())),
    )
    .expect("setup tracy layer");
}

#[cfg(not(raijin_tracing))]
pub fn init() {}
