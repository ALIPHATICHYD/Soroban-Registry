use crate::ai::service::{ChatMessage, ContractContext};
use serde_json::Value;

/// Builds context-aware prompts for AI chat interactions
pub struct PromptBuilder;

impl PromptBuilder {
    /// Build a system prompt for general contract Q&A
    pub fn build_system_prompt() -> String {
        r#"
You are a senior Soroban smart contract engineer and security expert. 
Soroban is Stellar's smart contract platform, using Rust-based contracts.

Key principles you follow:
1. Security first - always mention potential vulnerabilities
2. Gas efficiency - suggest optimizations
3. Best practices - reference official Soroban SDK patterns
4. Clarity - explain complex concepts simply

When responding:
- Use markdown formatting
- Include code examples with proper syntax highlighting
- For vulnerabilities, rate severity: critical/high/medium/low
- For code suggestions, show complete corrected snippets
- If uncertain, state your assumptions clearly
"#
        .trim()
        .to_string()
    }

    /// Build prompt for contract analysis
    pub fn build_analysis_prompt(
        contract_code: &str,
        contract_name: &str,
        description: Option<&str>,
        category: Option<&str>,
        tags: &[String],
    ) -> String {
        format!(
            r#"Analyze this Soroban smart contract:

**Contract Name:** {}
**Description:** {}
**Category:** {}
**Tags:** {}

```rust
{}
```

Provide a comprehensive analysis with these sections:

## 1. Summary
Brief (2-3 sentences) overview of what this contract does.

## 2. Key Functions
List and explain the main public functions and their purposes.

## 3. Security Analysis
- **Critical/High vulnerabilities** (if any)
- **Medium/Low concerns**
- **Best practice violations**

## 4. Gas Optimization
Suggest specific gas-saving improvements (if applicable).

## 5. Upgradeability & Maintenance
Notes on upgradeability patterns used and maintenance considerations.

## 6. Recommendations
Top 3-5 actionable improvements.
"#,
            contract_name,
            description.unwrap_or("No description"),
            category.unwrap_or("Uncategorized"),
            tags.join(", "),
            contract_code
        )
    }

    /// Build prompt for vulnerability scanning
    pub fn build_vulnerability_prompt(contract_code: &str) -> String {
        format!(
            r#"Perform a comprehensive security audit of this Soroban contract:

```rust
{}
```

Check for the following vulnerability categories:

1. **Reentrancy & External Calls**
   - Unchecked call results
   - Reentrancy risks

2. **Access Control**
   - Missing authorization
   - Admin privilege escalation

3. **Arithmetic**
   - Overflow/underflow (note: Soroban has checked arithmetic by default but verify)
   - precision errors

4. **Logic Errors**
   - Missing validation
   - Incorrect conditions
   - Front-running vulnerabilities

5. **Storage & State**
   - Uninitialized storage
   - Race conditions

For each finding:
- Severity: critical/high/medium/low
- Location: line number or function
- Description: clear explanation
- Recommended fix: specific code change

Format as a markdown table with columns: Severity | Category | Location | Description.
"#,
            contract_code
        )
    }

    /// Build prompt for code explanation
    pub fn build_explanation_prompt(contract_code: &str, focus_area: Option<&str>) -> String {
        let focus_clause = match focus_area {
            Some(area) => format!("Focus on: {}", area),
            None => "Provide a general overview".to_string(),
        };

        format!(
            r#"Explain this Soroban contract code:

```rust
{}
```

{}
Use a beginner-friendly tone. Break down:
- What each function does
- Important Soroban-specific patterns (e.g., `Env`, `Symbol`, storage patterns)
- Any non-obvious logic

Prefer bullet points over paragraphs.
"#,
            contract_code, focus_clause
        )
    }

    /// Build prompt for code suggestions/help
    pub fn build_suggestion_prompt(
        contract_code: &str,
        user_request: &str,
        context: Option<&str>,
    ) -> String {
        format!(
            r#"I'm working on this Soroban contract:

```rust
{}
```

My question/request: {}

{}

Provide:
1. Direct answer or suggested code
2. Brief explanation of why this approach works
3. Any caveats or alternatives
"#,
            contract_code,
            user_request,
            context.map_or("".to_string(), |c| format!("Additional context: {}", c))
        )
    }

    /// Build prompt from chat history + new message
    pub fn build_chat_prompt(
        messages: &[ChatMessage],
        contract_context: Option<&ContractContext>,
    ) -> Vec<ChatMessage> {
        let mut final_messages = Vec::new();

        // System message with contract context (if available)
        if let Some(ctx) = contract_context {
            let system_content = format!(
                r#"You are assisting with a Soroban smart contract.

**Contract Details:**
- Name: {}
- Description: {}
- Category: {}
- Tags: {}

**Contract Code:**
```rust
{}
```

Provide accurate, helpful responses. For code suggestions, always include complete code snippets. 
When discussing security or optimization, be specific and reference Soroban best practices.
"#,
                ctx.contract_name,
                ctx.description.as_deref().unwrap_or("N/A"),
                ctx.category.as_deref().unwrap_or("N/A"),
                ctx.tags.join(", "),
                ctx.contract_code
            );
            final_messages.push(ChatMessage {
                role: "system".to_string(),
                content: system_content,
            });
        } else {
            final_messages.push(ChatMessage {
                role: "system".to_string(),
                content: Self::build_system_prompt(),
            });
        }

        // Clone conversation history
        final_messages.extend(messages.iter().cloned());

        final_messages
    }

    /// Parse user query to detect intent
    /// Returns (is_analysis, is_vulnerability, is_explanation, request_text)
    pub fn parse_query_intent(query: &str) -> (bool, bool, bool, String) {
        let lower = query.to_lowercase();

        let is_analysis = lower.contains("analyze")
            || lower.contains("review")
            || lower.contains("overview")
            || lower.contains("summary");

        let is_vulnerability = lower.contains("vulnerab")
            || lower.contains("security")
            || lower.contains("audit")
            || lower.contains("exploit")
            || lower.contains("risk");

        let is_explanation = lower.contains("explain")
            || lower.contains("understand")
            || lower.contains("how does")
            || lower.contains("what does");

        let request_text = if is_analysis || is_vulnerability || is_explanation {
            // These are handled specially, keep full text
            query.to_string()
        } else {
            // Direct question/code help
            query.to_string()
        };

        (is_analysis, is_vulnerability, is_explanation, request_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_intent() {
        let (_, v, _, _) =
            PromptBuilder::parse_query_intent("What are the security vulnerabilities?");
        assert!(v, "Should detect vulnerability");

        let (a, _, _, _) = PromptBuilder::parse_query_intent("Analyze this contract");
        assert!(a, "Should detect analysis intent");

        let (_, _, e, _) = PromptBuilder::parse_query_intent("Explain the transfer function");
        assert!(e, "Should detect explanation intent");

        let (a, v, e, _) = PromptBuilder::parse_query_intent("How do I add a multisig?");
        assert!(!a && !v && !e, "General question");
    }
}
