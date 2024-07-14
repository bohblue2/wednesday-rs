use serde::Serialize;
use wednesday_macro::AsUrlParams;

// Define a struct to test AsUrlParams
#[derive(Serialize, AsUrlParams)]
struct UrlParam {
    pub field1: String,
    pub field3: i32,
}

// ... existing code ...

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_url_params() {
        let params = UrlParam {
            field1: "value1".to_string(),
            field3: 42,
        };
        let url_params = params.to_url_params();
        assert_eq!(url_params, "field1=value1&field3=42");
    }

    #[test]
    fn test_as_url_params_with_null() {
        let params = UrlParam {
            field1: "value1".to_string(),
            field3: 42,
        };
        let url_params = params.to_url_params();
        assert_eq!(url_params, "field1=value1&field3=42");
    }
}
