// Импортируем необходимые модули и типы из крейтов alloy и стандартной библиотеки Rust.
use alloy:: providers::{ ProviderBuilder, Provider}; // ProviderBuilder для создания провайдера, Provider для его использования.
use alloy_primitives::{address}; // Тип 'address' для работы с адресами Ethereum.
use alloy_transport_ws::WsConnect; // Модуль для установки WebSocket-соединения.
use alloy_sol_types::sol; // Макрос 'sol!' для генерации Rust-биндингов из Solidity ABI.

use std::error::Error; // Стандартный трейт для обработки ошибок.
use std::sync::Arc; // Arc (Atomic Reference Count) для безопасного совместного владения провайдером в асинхронном коде.


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


 #[tokio::main] //- атрибут, который превращает асинхронную функцию main в синхронную точку входа
// и запускает Tokio-рантайм, необходимый для асинхронных операций.
async fn main() -> eyre::Result<(), Box<dyn Error>> {
    // Инициализация логирования с помощью tracing-subscriber.
    // Полезно для отладки, выводя сообщения о работе программы.
    tracing_subscriber::fmt::init();

    // Определяем URL WebSocket RPC
    let rpc_url = "wss://ethereum-rpc.publicnode.com";

 
    println!("Подключаемся к RPC-узлу по WebSocket: {}", rpc_url);
    
    // Создаем WebSocket-транспорт для подключения к RPC-узлу.
    let ws_transport = WsConnect::new(rpc_url);
    
    // Создаем провайдер, который будет использовать WebSocket-транспорт.
    // ProviderBuilder::new() создает билдер.
    // .connect_ws(ws_transport) устанавливает WebSocket-соединение.
    // .await? дожидается завершения асинхронной операции подключения
    // и возвращает ошибку, если она произошла (благодаря '?' оператору).
    let connected_provider = ProviderBuilder::new()
        .connect_ws(ws_transport)
        .await?;
    
    // Оборачиваем провайдер в Arc. Это позволяет безопасно делиться владением
    // провайдером между несколькими асинхронными задачами или потоками.
    let provider = Arc::new(connected_provider); 

    // Сообщение об успешном подключении.
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