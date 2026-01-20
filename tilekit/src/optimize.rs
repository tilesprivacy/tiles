use crate::modelfile::Modelfile;
use bon::Builder;
use dspy_rs::{
    COPRO, ChatAdapter, Evaluator, Example, LM, MetaSignature, Module, Optimizable, Optimizer,
    Predict, Prediction, Predictor, Signature, configure, example,
};
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;

#[derive(Debug, Deserialize)]
struct TrainingExample {
    input: String,
    output: String,
}

#[Signature]
pub struct SystemPromptSignature {
    /// Act as a task-specific assistant based on the instructions provided.
    #[input]
    pub user_input: String,
    #[output]
    pub ai_response: String,
}

#[Signature]
pub struct SyntheticDataSignature {
    /// You are a data generator. Given a SYSTEM prompt, generate a JSON array of 5 diverse and representative training examples.
    /// Each example must be a JSON object with EXACTLY two fields: "input" (the user's query) and "output" (the expected AI response).
    #[input]
    pub system_prompt: String,
    #[output]
    /// A JSON array like: [{"input": "...", "output": "..."}, ...]
    pub synthetic_data: String,
}

#[derive(Builder)]
pub struct PromptOptimizerModule {
    #[builder(default = Predict::new(SystemPromptSignature::new()))]
    pub predictor: Predict,
}

impl Module for PromptOptimizerModule {
    async fn forward(&self, inputs: Example) -> anyhow::Result<Prediction> {
        self.predictor.forward(inputs).await
    }
}

impl Optimizable for PromptOptimizerModule {
    fn parameters(&mut self) -> IndexMap<String, &mut dyn Optimizable> {
        let mut params: IndexMap<String, &mut dyn Optimizable> = IndexMap::new();
        params.insert("predictor".to_string(), &mut self.predictor);
        params
    }
}

impl Evaluator for PromptOptimizerModule {
    async fn metric(&self, example: &Example, prediction: &Prediction) -> f32 {
        let ai_response_field = prediction.get("ai_response", None);
        let ai_response = ai_response_field.as_str().unwrap_or("");

        let ground_truth_field = example.get("ai_response", None);
        let ground_truth = ground_truth_field.as_str().unwrap_or("");

        let mut score = 0.0;

        // 1. Correctness Signal: Similarity to ground truth (Dice coefficient)
        if !ground_truth.is_empty() {
            let pred_tokens: HashSet<_> = ai_response.split_whitespace().collect();
            let gt_tokens: HashSet<_> = ground_truth.split_whitespace().collect();

            if !pred_tokens.is_empty() && !gt_tokens.is_empty() {
                let intersection = pred_tokens.intersection(&gt_tokens).count();
                let similarity =
                    2.0 * (intersection as f32) / ((pred_tokens.len() + gt_tokens.len()) as f32);

                // Boost for exact match
                if ai_response.trim() == ground_truth.trim() {
                    score += 0.5;
                } else {
                    score += similarity * 0.4;
                }
            }
        }

        // 2. Formatting & Persona Heuristics (weighted lower now that we have ground truth)

        // Reward non-empty responses
        if !ai_response.is_empty() {
            score += 0.1;
        }

        // Reward reasonable length
        let len = ai_response.len();
        if len > 50 && len < 1000 {
            score += 0.1;
        }

        // Reward structure (presence of newlines or bullet points often indicate better prompts/responses)
        if ai_response.contains('\n') || ai_response.contains('-') || ai_response.contains('*') {
            score += 0.1;
        }

        // Reward persona-like language
        let lower = ai_response.to_lowercase();
        if lower.contains("you are") || lower.contains("act as") || lower.contains("assistant") {
            score += 0.2;
        }

        score
    }
}

pub async fn optimize(
    modelfile_path: String,
    data_path: Option<String>,
    model: String,
) -> Result<Modelfile, String> {
    println!("Optimizing Modelfile: {}", modelfile_path);

    // 1. Read Modelfile
    let content = fs::read_to_string(&modelfile_path)
        .map_err(|e| format!("Error reading Modelfile: {}", e))?;

    let mut modelfile: Modelfile = content
        .parse()
        .map_err(|e| format!("Error parsing Modelfile: {}", e))?;

    let system_prompt = modelfile.system.clone().unwrap_or_default();
    if system_prompt.trim().is_empty() {
        return Err(
            "Error: The Modelfile has an empty SYSTEM prompt. Optimization requires a starting prompt to understand the task objective.".to_string()
        );
    }
    println!("Current SYSTEM prompt: \"{}\"", system_prompt);

    // 2. Configure DSRs
    let lm = LM::builder().model(model).build().await.map_err(|e| {
        format!(
            "Error configuring LM: {}. Make sure appropriate API keys are set.",
            e
        )
    })?;

    configure(lm, ChatAdapter);

    // 3. Load or Generate Training Data
    let examples = if let Some(path) = data_path {
        println!("Loading training data from: {}", path);
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Error reading data file {}: {}", path, e))?;

        let data: Vec<TrainingExample> = serde_json::from_str(&content)
            .map_err(|e| format!("Error parsing data file {}: {}", path, e))?;

        data.into_iter()
            .map(|e| {
                example! {
                    "user_input": "input" => e.input,
                    "ai_response": "output" => e.output,
                }
            })
            .collect()
    } else {
        println!("No training data provided. Generating synthetic examples...");
        generate_synthetic_examples(&system_prompt)
            .await
            .map_err(|e| format!("Failed to generate synthetic examples: {}", e))?
    };

    if examples.is_empty() {
        return Err("No training examples available. Cannot optimize effectively.".to_string());
    }

    // 4. Run COPRO Optimizer
    println!(
        "Running COPRO optimizer with {} examples...",
        examples.len()
    );

    let mut sig = SystemPromptSignature::new();
    sig.update_instruction(system_prompt.clone()).map_err(|e| {
        format!(
            "Error setting initial system prompt: {}. Prompt: \"{}\"",
            e, system_prompt
        )
    })?;

    let mut module = PromptOptimizerModule::builder()
        .predictor(Predict::new(sig))
        .build();

    let optimizer = COPRO::builder().breadth(5).depth(2).build();

    optimizer
        .compile(&mut module, examples)
        .await
        .map_err(|e| format!("Optimization failed: {}", e))?;

    let optimized_instructions = module.predictor.get_signature().instruction();
    println!("Optimization complete!");
    println!("New SYSTEM prompt: \n{}", optimized_instructions);

    // 5. Update Modelfile
    let _ = modelfile.add_system(&optimized_instructions);

    Ok(modelfile)
}

async fn generate_synthetic_examples(system_prompt: &str) -> anyhow::Result<Vec<Example>> {
    let predictor = Predict::new(SyntheticDataSignature::new());
    let input = example! {
        "system_prompt": "input" => system_prompt.to_string(),
    };

    let prediction = predictor.forward(input).await?;
    let field = prediction.get("synthetic_data", None);
    let synthetic_json = field.as_str().unwrap_or("");

    // Clean up potential markdown formatting
    let clean_json = synthetic_json
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let data: Vec<TrainingExample> = serde_json::from_str(clean_json)?;

    Ok(data
        .into_iter()
        .map(|e| {
            example! {
                "user_input": "input" => e.input,
                "ai_response": "output" => e.output,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dspy_rs::{LmUsage, Prediction};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_metric_exact_match() {
        let module = PromptOptimizerModule::builder().build();
        let example = example! {
            "ai_response" : "output" => "Hello world",
        };
        let mut data = HashMap::new();
        data.insert("ai_response".to_string(), "Hello world".into());
        let prediction = Prediction::new(data, LmUsage::default());

        let score = module.metric(&example, &prediction).await;
        assert!(score >= 0.6);
    }

    #[tokio::test]
    async fn test_metric_no_match() {
        let module = PromptOptimizerModule::builder().build();
        let example = example! {
            "ai_response" : "output" => "Hello world",
        };
        let mut data = HashMap::new();
        data.insert("ai_response".to_string(), "Goodbye universe".into());
        let prediction = Prediction::new(data, LmUsage::default());

        let score = module.metric(&example, &prediction).await;
        assert!(score <= 0.2);
    }

    #[tokio::test]
    async fn test_metric_persona() {
        let module = PromptOptimizerModule::builder().build();
        let example = example! {
            "ai_response" : "output" => "Hello",
        };
        let mut data = HashMap::new();
        data.insert("ai_response".to_string(), "Act as an assistant".into());
        let prediction = Prediction::new(data, LmUsage::default());

        let score = module.metric(&example, &prediction).await;
        assert!(score >= 0.3);
    }

    #[tokio::test]
    async fn test_metric_empty_response() {
        let module = PromptOptimizerModule::builder().build();
        let example = example! {
            "ai_response" : "output" => "Hello world",
        };
        let mut data = HashMap::new();
        data.insert("ai_response".to_string(), "".into());
        let prediction = Prediction::new(data, LmUsage::default());

        let score = module.metric(&example, &prediction).await;
        assert!(score == 0.0);
    }

    #[tokio::test]
    async fn test_metric_partial_match() {
        let module = PromptOptimizerModule::builder().build();
        let example = example! {
            "ai_response" : "output" => "Hello world how are you",
        };
        let mut data = HashMap::new();
        data.insert("ai_response".to_string(), "Hello world".into());
        let prediction = Prediction::new(data, LmUsage::default());

        let score = module.metric(&example, &prediction).await;
        // Partial match should score between 0 and exact match
        assert!(score > 0.0 && score < 0.6);
    }

    #[tokio::test]
    async fn test_metric_structure_bonus() {
        let module = PromptOptimizerModule::builder().build();
        let example = example! {
            "ai_response" : "output" => "test",
        };
        let mut data = HashMap::new();
        data.insert(
            "ai_response".to_string(),
            "Line 1\nLine 2\n- bullet point".into(),
        );
        let prediction = Prediction::new(data, LmUsage::default());

        let score = module.metric(&example, &prediction).await;
        // Should get structure bonus for newlines and bullet points
        assert!(score >= 0.2);
    }

    #[tokio::test]
    async fn test_optimize_missing_file() {
        let result = optimize(
            "nonexistent_file.modelfile".to_string(),
            None,
            "openai:gpt-4o-mini".to_string(),
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Error reading Modelfile"));
    }

    #[tokio::test]
    async fn test_optimize_empty_system_prompt() {
        // Create a temp file with no system prompt
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_empty_system.modelfile");
        std::fs::write(&temp_file, "FROM llama3.2\n").unwrap();

        let result = optimize(
            temp_file.to_string_lossy().to_string(),
            None,
            "openai:gpt-4o-mini".to_string(),
        )
        .await;

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty SYSTEM prompt"));
    }
}
