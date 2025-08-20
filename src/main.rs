#[cfg(feature = "telemetry")]
use opentelemetry::trace::Span;
#[cfg(feature = "telemetry")]
use dotenv::dotenv;
#[cfg(feature = "telemetry")]
use opentelemetry::trace::Tracer;
// Импортируем необходимые модули и типы из крейтов alloy и стандартной библиотеки Rust.
use alloy::providers::{ProviderBuilder, Provider}; // ProviderBuilder для создания провайдера, Provider для его использования.
use alloy_primitives::{address}; // Тип 'address' для работы с адресами Ethereum.
use alloy_transport_ws::WsConnect; // Модуль для установки WebSocket-соединения.
use alloy_sol_types::sol; // Макрос 'sol!' для генерации Rust-биндингов из Solidity ABI.

use std::sync::Arc; // Arc (Atomic Reference Count) для безопасного совместного владения провайдером в асинхронном коде.
//________________________________________________________________________________________________________
// Импорт необходимых модулей и типов.

#[cfg(feature = "telemetry")]
mod telemetry;
#[cfg(feature = "telemetry")]
use telemetry::init_tracer;
#[cfg(feature = "telemetry")]
use opentelemetry::global;
#[cfg(feature = "telemetry")]
use opentelemetry::KeyValue;
#[cfg(feature = "telemetry")]
use opentelemetry::global::shutdown_tracer_provider;

// ...existing code...

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
async fn main() -> eyre::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    #[cfg(feature = "telemetry")]
    {
        dotenv().ok();
        let _ = init_tracer();
    }

    // --- 1. Получаем глобальный трейсер ---
    #[cfg(feature = "telemetry")]
    let tracer = global::tracer("main_tracer");
    
    // --- 2. Создаем спан для всей основной операции ---
    // Этот спан будет охватывать всю работу по подключению и вызову Multicall.
    #[cfg(feature = "telemetry")]
    let mut main_span = tracer.start("main_multicall_operation");
    
    // Оборачиваем весь код в `tokio::task::spawn_blocking` или используем `let _guard = main_span.set_current();`
    // для правильного контекста, но для простоты мы просто его "запустим".
    // В асинхронном коде Rust, чтобы контекст спана был доступен для вложенных вызовов,
    // вам нужно использовать `opentelemetry::Context` и `Span::enter()`, 
    // но для простого случая достаточно использовать `start` и `end`.

    // --- Начало вашей основной логики ---

    let rpc_url = "wss://ethereum-rpc.publicnode.com";
    println!("Подключаемся к RPC-узлу по WebSocket: {}", rpc_url);
    
    let ws_transport = WsConnect::new(rpc_url);
    
    let connected_provider = ProviderBuilder::new()
        .connect_ws(ws_transport)
        .await?;
    
    let provider = Arc::new(connected_provider); 
    
    println!(" ___OK___");
    
    let custom_oracle_address = address!("0x6CAFE228eC0B0bC2D076577d56D35Fe704318f6d");
    let oracle_contract = CustomOracle::new(custom_oracle_address, Arc::clone(&provider));

    // Добавляем событие в спан перед началом Multicall
    #[cfg(feature = "telemetry")]
    main_span.add_event("Starting multicall aggregate", vec![]);

    println!("\n--- Запрос оракула через Multicall (высокоуровневый API) ---");
    
    let price_call = oracle_contract.price();
    let base_feed_1_call = oracle_contract.BASE_FEED_1();
    let base_feed_2_call = oracle_contract.BASE_FEED_2();
    let quote_feed_1_call = oracle_contract.QUOTE_FEED_1();
    let quote_feed_2_call = oracle_contract.QUOTE_FEED_2();
    let scale_factor_call = oracle_contract.SCALE_FACTOR();
    let vault_call = oracle_contract.VAULT();
    let vault_conversion_sample_call = oracle_contract.VAULT_CONVERSION_SAMPLE();

    let multicall = provider
        .multicall()
        .add(price_call)
        .add(base_feed_1_call)
        .add(base_feed_2_call)
        .add(quote_feed_1_call)
        .add(quote_feed_2_call)
        .add(scale_factor_call)
        .add(vault_call)
        .add(vault_conversion_sample_call);

    // Эта асинхронная операция теперь выполняется внутри нашего спана!
    let (
        price,
        base_feed_1,
        base_feed_2,
        quote_feed_1,
        quote_feed_2,
        scale_factor,
        vault,
        vault_conversion_sample,
    ) = multicall.aggregate().await?;
    
    // Добавляем результат в спан как атрибуты, если это полезно
    #[cfg(feature = "telemetry")]
    {
        main_span.set_attribute(KeyValue::new("price", price.to_string()));
        main_span.set_attribute(KeyValue::new("scale_factor", scale_factor.to_string()));
        main_span.add_event("Multicall completed successfully", vec![]);
    }

    println!("  price: {}", price);
    println!("  BASE_FEED_1: {:?}", base_feed_1);
    // ... (остальные принты) ...
    
    // --- 3. Завершаем спан ---
    #[cfg(feature = "telemetry")]
    main_span.end();
    
    #[cfg(feature = "telemetry")]
    shutdown_tracer_provider();
    
    Ok(())
}