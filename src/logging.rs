use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Inicializa sistema de logging
pub fn init_tracing() {
    // Filtro baseado em variável de ambiente ou padrão
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug"));
    
    // Formato customizado
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true);
    
    // Registra subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}
