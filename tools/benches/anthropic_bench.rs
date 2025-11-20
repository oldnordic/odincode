//! Performance benchmarks for Anthropic provider
//!
//! This module provides comprehensive performance benchmarks for the Anthropic model provider,
//! including model listing, cache operations, and streaming functionality.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use odincode_tools::models::anthropic::AnthropicProvider;
use odincode_tools::models::interface::{ChatCompletionRequest, ChatMessage, MessageRole};
use tokio::runtime::Runtime;

/// Benchmark for Anthropic provider creation
fn bench_anthropic_provider_creation(c: &mut Criterion) {
    c.bench_function("anthropic_provider_creation", |b| {
        b.iter(|| {
            let provider = AnthropicProvider::new();
            black_box(provider);
        })
    });
}

/// Benchmark for Anthropic provider with API key
fn bench_anthropic_provider_with_api_key(c: &mut Criterion) {
    c.bench_function("anthropic_provider_with_api_key", |b| {
        b.iter(|| {
            let provider = AnthropicProvider::with_api_key("test-key".to_string());
            black_box(provider);
        })
    });
}

/// Benchmark for model info creation
fn bench_model_info_creation(c: &mut Criterion) {
    let provider = AnthropicProvider::new();

    c.bench_function("model_info_creation", |b| {
        b.iter(|| {
            let model_info = provider.create_model_info(
                black_box("claude-3-opus-20240229"),
                black_box("Claude 3 Opus"),
                black_box(200000),
            );
            black_box(model_info);
        })
    });
}

/// Benchmark for cache timeout operations
fn bench_cache_timeout_operations(c: &mut Criterion) {
    c.bench_function("cache_timeout_operations", |b| {
        b.iter(|| {
            let mut provider = AnthropicProvider::new();
            provider.set_cache_timeout(black_box(60));
            black_box(provider.cache_timeout);
        })
    });
}

/// Benchmark for model list fetching (cached)
fn bench_model_list_fetching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("model_list_fetching", |b| {
        b.to_async(&rt).iter(|| async {
            let provider = AnthropicProvider::new();
            let models = provider.fetch_models().await.unwrap();
            black_box(models);
        })
    });
}

/// Benchmark for cache validation
fn bench_cache_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("cache_validation", |b| {
        b.to_async(&rt).iter(|| async {
            let provider = AnthropicProvider::new();
            let is_valid = provider.is_cache_valid().await;
            black_box(is_valid);
        })
    });
}

/// Benchmark for cache operations
fn bench_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("cache_operations", |b| {
        b.to_async(&rt).iter(|| async {
            let provider = AnthropicProvider::new();
            let models = provider.fetch_models().await.unwrap();

            // Benchmark cache update
            provider.update_cache(models.clone()).await;

            // Benchmark cache clear
            provider.clear_cache().await;

            black_box(());
        })
    });
}

/// Benchmark for chat completion request creation
fn bench_chat_completion_request_creation(c: &mut Criterion) {
    c.bench_function("chat_completion_request_creation", |b| {
        b.iter(|| {
            let request = ChatCompletionRequest {
                model: black_box("claude-3-opus-20240229".to_string()),
                messages: vec![ChatMessage {
                    role: MessageRole::User,
                    content: black_box("Hello, how are you?".to_string()),
                    name: None,
                    function_call: None,
                }],
                max_tokens: Some(black_box(1024)),
                temperature: Some(black_box(0.7)),
                top_p: Some(black_box(1.0)),
                stop: None,
            };
            black_box(request);
        })
    });
}

/// Benchmark for provider availability check
fn bench_provider_availability_check(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("provider_availability_check", |b| {
        b.to_async(&rt).iter(|| async {
            let provider = AnthropicProvider::new();
            let is_available = provider.is_available().await;
            black_box(is_available);
        })
    });
}

/// Benchmark for model listing with cache
fn bench_model_listing_with_cache(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("model_listing_with_cache", |b| {
        b.to_async(&rt).iter(|| async {
            let provider = AnthropicProvider::new();

            // First call (should fetch from API)
            let models1 = provider.list_models().await.unwrap();

            // Second call (should use cache)
            let models2 = provider.list_models().await.unwrap();

            black_box((models1, models2));
        })
    });
}

/// Benchmark for memory usage of model info structures
fn bench_model_info_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_info_memory_usage");

    group.throughput(Throughput::Elements(1));

    group.bench_function("single_model_info", |b| {
        b.iter(|| {
            let provider = AnthropicProvider::new();
            let model_info = provider.create_model_info(
                black_box("claude-3-opus-20240229"),
                black_box("Claude 3 Opus"),
                black_box(200000),
            );
            black_box(model_info);
        })
    });

    group.bench_function("multiple_model_info", |b| {
        b.iter(|| {
            let provider = AnthropicProvider::new();
            let models: Vec<_> = (0..10)
                .map(|i| {
                    provider.create_model_info(
                        black_box(&format!("model-{}", i)),
                        black_box(&format!("Model {}", i)),
                        black_box(200000),
                    )
                })
                .collect();
            black_box(models);
        })
    });

    group.finish();
}

/// Benchmark for concurrent model listing
fn bench_concurrent_model_listing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_model_listing", |b| {
        b.to_async(&rt).iter(|| async {
            let provider = AnthropicProvider::new();

            // Spawn multiple concurrent requests
            let handles: Vec<_> = (0..5)
                .map(|_| {
                    let provider = provider.clone();
                    tokio::spawn(async move { provider.list_models().await })
                })
                .collect();

            // Wait for all to complete
            let results = futures::future::join_all(handles).await;
            black_box(results);
        })
    });
}

criterion_group!(
    benches,
    bench_anthropic_provider_creation,
    bench_anthropic_provider_with_api_key,
    bench_model_info_creation,
    bench_cache_timeout_operations,
    bench_model_list_fetching,
    bench_cache_validation,
    bench_cache_operations,
    bench_chat_completion_request_creation,
    bench_provider_availability_check,
    bench_model_listing_with_cache,
    bench_model_info_memory_usage,
    bench_concurrent_model_listing
);

criterion_main!(benches);
