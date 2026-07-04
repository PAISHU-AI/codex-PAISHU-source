use crate::models::TokenBreakdown;

#[derive(Debug, Clone)]
pub struct ModelTokenPrice {
    pub input_per_million: f64,
    pub cached_input_per_million: f64,
    pub output_per_million: f64,
}

pub fn model_token_price(model: Option<&str>) -> ModelTokenPrice {
    let normalized = model.unwrap_or_default().to_ascii_lowercase();
    if normalized.contains("gpt-5.5-pro") {
        return ModelTokenPrice {
            input_per_million: 30.0,
            cached_input_per_million: 30.0,
            output_per_million: 180.0,
        };
    }
    if normalized.contains("gpt-5.5") || normalized == "chat-latest" {
        return ModelTokenPrice {
            input_per_million: 5.0,
            cached_input_per_million: 0.5,
            output_per_million: 30.0,
        };
    }
    if normalized.contains("gpt-5.4-mini") {
        return ModelTokenPrice {
            input_per_million: 0.75,
            cached_input_per_million: 0.075,
            output_per_million: 4.5,
        };
    }
    if normalized.contains("gpt-5.4-nano") {
        return ModelTokenPrice {
            input_per_million: 0.2,
            cached_input_per_million: 0.02,
            output_per_million: 1.25,
        };
    }
    if normalized.contains("gpt-5.4-pro") {
        return ModelTokenPrice {
            input_per_million: 30.0,
            cached_input_per_million: 30.0,
            output_per_million: 180.0,
        };
    }
    if normalized.contains("gpt-5.4") {
        return ModelTokenPrice {
            input_per_million: 2.5,
            cached_input_per_million: 0.25,
            output_per_million: 15.0,
        };
    }
    if normalized.contains("gpt-5.3-codex")
        || normalized.contains("gpt-5.2-codex")
        || normalized.contains("gpt-5.3-chat")
        || normalized.contains("gpt-5.2")
    {
        return ModelTokenPrice {
            input_per_million: 1.75,
            cached_input_per_million: 0.175,
            output_per_million: 14.0,
        };
    }
    if normalized.contains("gpt-5") {
        return ModelTokenPrice {
            input_per_million: 1.25,
            cached_input_per_million: 0.125,
            output_per_million: 10.0,
        };
    }
    ModelTokenPrice {
        input_per_million: 5.0,
        cached_input_per_million: 0.5,
        output_per_million: 30.0,
    }
}

pub fn estimated_cost_usd(tokens: TokenBreakdown, price: &ModelTokenPrice) -> f64 {
    tokens.uncached_input_tokens() as f64 / 1_000_000.0 * price.input_per_million
        + tokens.billable_cached_input_tokens() as f64 / 1_000_000.0
            * price.cached_input_per_million
        + tokens.output_tokens.max(0) as f64 / 1_000_000.0 * price.output_per_million
}

pub fn estimated_aggregate_cost_usd(total_tokens: i64) -> f64 {
    let price = model_token_price(Some("gpt-5"));
    total_tokens.max(0) as f64 / 1_000_000.0 * price.input_per_million
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_cached_and_uncached_tokens() {
        let cost = estimated_cost_usd(
            TokenBreakdown {
                input_tokens: 1_000_000,
                cached_input_tokens: 400_000,
                output_tokens: 100_000,
                reasoning_output_tokens: 0,
                total_tokens: 1_100_000,
            },
            &model_token_price(Some("chat-latest")),
        );
        assert!((cost - 6.2).abs() < 0.001);
    }

    #[test]
    fn estimates_aggregate_tokens_from_gpt5_input_rate() {
        assert!((estimated_aggregate_cost_usd(2_000_000) - 2.5).abs() < 0.001);
    }

    #[test]
    fn prices_codex_models_with_official_input_and_output_rates() {
        let price = model_token_price(Some("gpt-5.3-codex"));
        assert_eq!(price.input_per_million, 1.75);
        assert_eq!(price.cached_input_per_million, 0.175);
        assert_eq!(price.output_per_million, 14.0);
    }
}
