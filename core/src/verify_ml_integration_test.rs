#[cfg(test)]
mod tests {
    use super::verify_ml_integration::verify_ml_integration;

    #[test]
    fn test_ml_integration_verification() {
        // This test verifies that all ML models return actual predictions, not zeros
        // Note: This test may take some time to run as it trains all models
        
        let result = verify_ml_integration();
        assert!(result.is_ok(), "ML Integration verification failed: {:?}", result);
    }
}