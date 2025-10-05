use ai::model::Model;

#[test]
fn test_custom_model_creation() {
    // Test that custom model names are accepted
    let custom_model: Model = "custom-gpt-model".into();
    match custom_model {
        Model::Custom(name) => assert_eq!(name, "custom-gpt-model"),
        _ => panic!("Should create Custom variant")
    }
}

#[test]
fn test_predefined_model_creation() {
    // Test that predefined models still work
    let gpt4: Model = "gpt-4".into();
    assert_eq!(gpt4, Model::GPT4);
    
    let gpt4o: Model = "gpt-4o".into();
    assert_eq!(gpt4o, Model::GPT4o);
}

#[test]
fn test_model_string_conversion() {
    let custom_model = Model::Custom("my-custom-model".to_string());
    let model_str = String::from(&custom_model);
    assert_eq!(model_str, "my-custom-model");
    
    let gpt4 = Model::GPT4;
    let gpt4_str = String::from(&gpt4);
    assert_eq!(gpt4_str, "gpt-4");
}

#[test]
fn test_model_display() {
    let custom_model = Model::Custom("test-model".to_string());
    assert_eq!(format!("{}", custom_model), "test-model");
}

#[test]
fn test_custom_model_context_size() {
    let custom_model = Model::Custom("unknown-model".to_string());
    // Should use default context size for unknown models
    assert_eq!(custom_model.context_size(), 128000);
}

#[test]
fn test_custom_model_token_counting() {
    let custom_model = Model::Custom("test-model".to_string());
    let result = custom_model.count_tokens("Hello world");
    assert!(result.is_ok(), "Token counting should work for custom models");
    assert!(result.unwrap() > 0, "Should count tokens");
}

#[test]
fn test_from_str_parsing() {
    // Test parsing various model names
    let model1: Result<Model, _> = "gpt-4o".parse();
    assert!(model1.is_ok());
    assert_eq!(model1.unwrap(), Model::GPT4o);
    
    let model2: Result<Model, _> = "custom-model-name".parse();
    assert!(model2.is_ok());
    match model2.unwrap() {
        Model::Custom(name) => assert_eq!(name, "custom-model-name"),
        _ => panic!("Should create Custom variant")
    }
}