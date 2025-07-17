// Импортируем необходимые модули и типы из крейтов alloy и стандартной библиотеки Rust.
use alloy:: providers::{ ProviderBuilder, Provider}; // ProviderBuilder для создания провайдера, Provider для его использования.
use alloy_primitives::{address}; // Тип 'address' для работы с адресами Ethereum.
use alloy_transport_ws::WsConnect; // Модуль для установки WebSocket-соединения.
use alloy_sol_types::sol; // Макрос 'sol!' для генерации Rust-биндингов из Solidity ABI.

use std::error::Error; // Стандартный трейт для обработки ошибок.
use std::sync::Arc; // Arc (Atomic Reference Count) для безопасного совместного владения провайдером в асинхронном коде.
//________________________________________________________________________________________________________
use opentelemetry::global::shutdown_tracer_provider;
use opentelemetry::sdk::Resource;
use opentelemetry::trace::TraceError;
use opentelemetry::{
    global, sdk::trace as sdktrace,
    trace::{TraceContextExt, Tracer},
    Context, Key, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;

use dotenv::dotenv;


fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let signoz_endpoint = std::env::var("SIGNOZ_ENDPOINT").expect("SIGNOZ_ENDPOINT not set");
    
    // Add /v1/traces path for HTTP OTLP endpoint
    let http_endpoint = if signoz_endpoint.ends_with("/v1/traces") {
        signoz_endpoint
    } else {
        format!("{}/v1/traces", signoz_endpoint.trim_end_matches('/'))
    };
    
    println!("Connecting to SigNoz at: {}", http_endpoint);
    
    // Create HTTP exporter instead of gRPC
    let exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_endpoint(http_endpoint);
    
    // For HTTP, we need to add headers differently
    let pipeline = opentelemetry_otlp::new_pipeline().tracing();
    
    // Add API key if provided (for secured SigNoz instances)
    if let Ok(api_key) = std::env::var("SIGNOZ_API_KEY") {
        // For HTTP, headers are typically added via environment variables or directly in requests
        unsafe {
            std::env::set_var("OTEL_EXPORTER_OTLP_HEADERS", format!("signoz-ingestion-key={}", api_key));
        }
        println!("Using API key authentication");
    }
    
    pipeline
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new(
                    opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                    std::env::var("APP_NAME").unwrap_or_else(|_| "chainlink_multicall_signoz".to_string()),
                ),
            ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)
}
//_____________________________________________________________________________________________________
// --- 1. Генерируем Rust-биндинги для вашего оракула ---
// Макрос 'sol!' читает переданный ему код Solidity (или его часть, описывающую интерфейс)
// и генерирует соответствующие структуры и методы на Rust.
sol! {
    #[sol(rpc)] // Атрибут #[sol(rpc)] указывает, что должны быть сгенерированы методы для вызова функций контракта через RPC.
    contract CustomOracle { // Объявляем интерфейс Solidity-контракта.
        // Ниже идут объявления функций контракта оракула, которые мы хотим вызывать.
        // 'external view returns (address)' означает, что функция внешняя (доступна извне),
        // только для view (не меняет состояние блокчейна) и возвращает адрес.
        function BASE_FEED_1() external view returns (address);
        function BASE_FEED_2() external view returns (address);
        function QUOTE_FEED_1() external view returns (address);
        function QUOTE_FEED_2() external view returns (address);
        function SCALE_FACTOR() external view returns (uint256); // Возвращает беззнаковое 256-битное целое число.
        function VAULT() external view returns (address);
        function VAULT_CONVERSION_SAMPLE() external view returns (uint256);
        function price() external view returns (uint256); // Основная функция, возвращающая цену.
    }
}


 #[tokio::main] 
async fn main() -> eyre::Result<(), Box<dyn Error>> {

    // Инициализация логирования с помощью tracing-subscriber.
    tracing_subscriber::fmt::init();
//_____________________________________________________________________________________________________
    dotenv().ok();
let _ = init_tracer();

  let tracer = global::tracer("global_tracer");
    let _cx = Context::new();
  
    tracer.in_span("operation", |cx| {
        let span = cx.span();
        span.set_attribute(Key::new("KEY").string("value"));

        span.add_event(
            "Operations",
            vec![
                Key::new("SigNoz is").string("Awesome"),
            ],
        );
    });
    shutdown_tracer_provider();
//_____________________________________________________________________________________________________

    // Определяем URL WebSocket RPC
    let rpc_url = "wss://ethereum-rpc.publicnode.com";
    println!("Подключаемся к RPC-узлу по WebSocket: {}", rpc_url);
    
   //созд WS-транспорт
    let ws_transport = WsConnect::new(rpc_url);
    
    // Создаем провайдер, который будет использовать WebSocket-транспорт.
    // ProviderBuilder::new() создает билдер.
    // .connect_ws(ws_transport) устанавливает WebSocket
    // .await? дожидается завершения операции подключения + ? Err

    let connected_provider = ProviderBuilder::new()
        .connect_ws(ws_transport)
        .await?;
    
    // Оборачиваем провайдер в Arc, безопасно делиться владением
    // между несколькими задачами или потоками.
    let provider = Arc::new(connected_provider); 

    println!(" ___OK___");

    // Определяем адрес контракта оракула в сети Ethereum.
    let custom_oracle_address = address!("0x6CAFE228eC0B0bC2D076577d56D35Fe704318f6d");
    
    // Создаем экземпляр контракта 'CustomOracle', используя сгенерированные биндинги,
    // адрес контракта и провайдер.
    let oracle_contract = CustomOracle::new(custom_oracle_address, Arc::clone(&provider));

    // Заголовок для вывода информации о Multicall-запросе.
    println!("\n--- Запрос оракула через Multicall (высокоуровневый API) ---");

    // Формируем отдельные вызовы для каждой функции оракула.
    // Например, 'oracle_contract.price()' возвращает объект вызова, который можно
    // использовать в Multicall.
    let price_call = oracle_contract.price();
    let base_feed_1_call = oracle_contract.BASE_FEED_1();
    let base_feed_2_call = oracle_contract.BASE_FEED_2();
    let quote_feed_1_call = oracle_contract.QUOTE_FEED_1();
    let quote_feed_2_call = oracle_contract.QUOTE_FEED_2();
    let scale_factor_call = oracle_contract.SCALE_FACTOR();
    let vault_call = oracle_contract.VAULT();
    let vault_conversion_sample_call = oracle_contract.VAULT_CONVERSION_SAMPLE();

    // Создаем Multicall билдер, используя метод '.multicall()' на провайдере.
    // Этот билдер позволяет нам добавлять несколько вызовов, которые будут
    // выполнены в одной агрегированной RPC-транзакции.
    let multicall = provider
        .multicall() // Инициализируем билдер Multicall.
        .add(price_call) // Добавляем вызов функции 'price()'.
        .add(base_feed_1_call) // Добавляем вызов 'BASE_FEED_1()'.
        .add(base_feed_2_call)
        .add(quote_feed_1_call)
        .add(quote_feed_2_call)
        .add(scale_factor_call)
        .add(vault_call)
        .add(vault_conversion_sample_call);

    // Выполняем агрегированный запрос к контракту Multicall.
    // .aggregate().await? отправляет все добавленные вызовы одной пачкой.
    // Alloy автоматически обрабатывает кодирование вызовов для Multicall
    // и декодирование всех результатов обратно в соответствующие Rust-типы.
    // Порядок возвращаемых значений строго соответствует порядку, в котором
    // вы добавляли вызовы с помощью '.add()'.
    let (
        price,
        base_feed_1,
        base_feed_2,
        quote_feed_1,
        quote_feed_2,
        scale_factor,
        vault,
        vault_conversion_sample,
    ) = multicall.aggregate().await?; // Используем '?' для обработки ошибок.

    
    println!("  price: {}", price);
    println!("  BASE_FEED_1: {:?}", base_feed_1); // Используем {:?} для форматирования адреса.
    println!("  BASE_FEED_2: {:?}", base_feed_2);
    println!("  QUOTE_FEED_1: {:?}", quote_feed_1);
    println!("  QUOTE_FEED_2: {:?}", quote_feed_2);
    println!("  SCALE_FACTOR: {}", scale_factor);
    println!("  VAULT: {:?}", vault);
    println!("  VAULT_CONVERSION_SAMPLE: {}", vault_conversion_sample);

    
    Ok(())
}