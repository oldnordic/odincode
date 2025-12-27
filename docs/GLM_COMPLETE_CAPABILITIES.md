# GLM (Z.AI) Complete Capabilities: Engineering Analysis

**Source**: [Z.AI Developer Documentation](https://docs.z.ai)
**Documented**: 2025-12-26
**Context**: OdinCode model-agnostic LLM integration (current test model)

---

## Executive Summary

GLM (Z.AI) provides **6 core capabilities** for production AI applications:

| Capability | Purpose | Model Support |
|------------|---------|---------------|
| **Deep Thinking** | Exposed reasoning chain | `glm-4-thinking` |
| **Thinking Mode** | Hybrid reasoning/response | `glm-4.5-air` |
| **Streaming Messages** | Real-time SSE output | `glm-4.7`, `glm-4.6`, `glm-4.5` |
| **Structured Output** | JSON mode guarantees | `glm-4.7`, `glm-4.6`, `glm-4.5` |
| **Context Caching** | Token/cost optimization | All GLM-4.x models |
| **Function Calling** | External tool integration | `glm-4-plus`, `glm-4.7`, `glm-4.6` |
| **Tool Streaming** | Streaming tool calls | `glm-4.6` only |

---

## Thinking Modes

### Mode 1: Interleaved Thinking (`type: "interleaved"`)

Default mode. Thoughts are mixed with the response in a single stream.

```python
response = client.chat.completions.create(
    model="glm-4-thinking",
    messages=[
        {"role": "user", "content": "Solve: What is 15% of 240?"}
    ],
    thinking={
        "type": "interleaved"  # Default
    }
)
```

**Output structure**:
```
<thinking>
Let me calculate this step by step.
15% means 15/100 = 0.15
240 × 0.15 = 36
</thinking>

The answer is 36.
```

**Use cases**: Problems requiring step-by-step reasoning visible to user.

---

### Mode 2: Preserved Thinking (`type: "preserved"`)

Thoughts are returned separately from the response content.

```python
response = client.chat.completions.create(
    model="glm-4-thinking",
    messages=[...],
    thinking={
        "type": "preserved"
    }
)

# Access thinking separately
thinking_content = response.choices[0].message.reasoning_content
answer_content = response.choices[0].message.content
```

**Structure**:
- `reasoning_content` — The thinking/thought chain
- `content` — The final answer only

**Use cases**: Internal reasoning, audit trails, debugging AI decisions.

---

### Mode 3: Turn-level Thinking (`type: "turn_level"`)

Each assistant turn produces thinking, regardless of whether tools are called.

```python
response = client.chat.completions.create(
    model="glm-4-thinking",
    messages=[...],
    thinking={
        "type": "turn_level"
    }
)
```

**Use cases**: Multi-turn conversations where reasoning on each turn is valuable.

---

## API Parameters

### thinking Object

```python
thinking={
    "type": "interleaved" | "preserved" | "turn_level"
}
```

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | Yes | Thinking mode: `"interleaved"`, `"preserved"`, or `"turn_level"` |

### Model Support

| Model | Deep Thinking Support |
|-------|----------------------|
| `glm-4-thinking` | ✅ Full support |
| `glm-4` | ❌ Not supported |
| `glm-4-flash` | ❌ Not supported |

**Critical**: Use `glm-4-thinking` model for Deep Thinking features.

---

## Response Structure

### Preserved Mode Response

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "The final answer text here...",
        "reasoning_content": "Step-by-step reasoning here...",
        "thinking_tokens": 127
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 50,
    "completion_tokens": 250,
    "total_tokens": 300
  }
}
```

### Interleaved Mode Response

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "<thinking>Reasoning...</thinking>\nFinal answer",
        "thinking_tokens": 127
      }
    }
  ]
}
```

---

## Code Examples

### Python SDK (Z.AI)

```python
from zhipuai import ZhipuAI

client = ZhipuAI(api_key="your-api-key")

def solve_with_thinking(question: str):
    response = client.chat.completions.create(
        model="glm-4-thinking",
        messages=[{"role": "user", "content": question}],
        thinking={"type": "preserved"}
    )

    return {
        "reasoning": response.choices[0].message.reasoning_content,
        "answer": response.choices[0].message.content,
        "thinking_tokens": response.choices[0].message.thinking_tokens
    }

result = solve_with_thinking(
    "Explain how Rust's ownership system prevents memory leaks"
)
print(f"Reasoning: {result['reasoning']}")
print(f"Answer: {result['answer']}")
```

### Java SDK

```java
import com.zhipu.ai.OpenApiClient;
import com.zhipu.ai.models.ChatCompletionRequest;
import com.zhipu.ai.models.ChatCompletionResponse;

public class ThinkingExample {
    public static void main(String[] args) {
        OpenApiClient client = new OpenApiClient("your-api-key");

        ChatCompletionRequest request = ChatCompletionRequest.builder()
            .model("glm-4-thinking")
            .messages(List.of(
                new Message("user", "Analyze this code's complexity")
            ))
            .thinking(ThinkingConfig.builder()
                .type("preserved")
                .build())
            .build();

        ChatCompletionResponse response = client.createChatCompletion(request);

        String reasoning = response.getChoices().get(0)
            .getMessage().getReasoningContent();
        String answer = response.getChoices().get(0)
            .getMessage().getContent();

        System.out.println("Reasoning: " + reasoning);
        System.out.println("Answer: " + answer);
    }
}
```

### OpenAI SDK (Compatible)

```python
from openai import OpenAI

# Z.AI API is OpenAI-compatible
client = OpenAI(
    api_key="your-glm-api-key",
    base_url="https://open.bigmodel.cn/api/paas/v4/"
)

response = client.chat.completions.create(
    model="glm-4-thinking",
    messages=[{"role": "user", "content": "Your question"}],
    extra_body={"thinking": {"type": "preserved"}}
)
```

---

## Function Calling with Deep Thinking

Deep Thinking integrates with GLM's function calling capability.

```python
import json

tools = [{
    "type": "function",
    "function": {
        "name": "search_codebase",
        "description": "Search the codebase for a pattern",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern (regex supported)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "File glob pattern (e.g., '*.rs')"
                }
            },
            "required": ["pattern"]
        }
    }
}]

response = client.chat.completions.create(
    model="glm-4-thinking",
    messages=[{
        "role": "user",
        "content": "Find all functions that return Result<T, E>"
    }],
    tools=tools,
    thinking={"type": "preserved"}
)

# Model reasons about which tool to use
reasoning = response.choices[0].message.reasoning_content
tool_call = response.choices[0].message.tool_calls[0]

print(f"Reasoning: {reasoning}")
print(f"Tool: {tool_call.function.name}")
print(f"Args: {tool_call.function.arguments}")
```

---

## Best Practices

### 1. Parameter Design

```python
# ✅ Good: Specific, typed parameters
{
    "name": "file_search",
    "parameters": {
        "pattern": {"type": "string", "minLength": 1},
        "max_results": {"type": "integer", "minimum": 1, "maximum": 100}
    }
}

# ❌ Bad: Vague, untyped parameters
{
    "name": "file_search",
    "parameters": {
        "query": {"type": "string"}  # What format? Regex? Glob?
    }
}
```

### 2. Error Handling

```python
try:
    response = client.chat.completions.create(
        model="glm-4-thinking",
        messages=messages,
        thinking={"type": "preserved"}
    )
except zhipuai.APIError as e:
    # Handle API errors
    print(f"API Error: {e}")
except zhipuai.RateLimitError as e:
    # Handle rate limiting
    print(f"Rate limited: {e}")
    time.sleep(60)
```

### 3. Input Validation

```python
def validate_thinking_request(messages: list, thinking_type: str):
    if not messages:
        raise ValueError("Messages cannot be empty")

    valid_types = ["interleaved", "preserved", "turn_level"]
    if thinking_type not in valid_types:
        raise ValueError(f"Invalid thinking type: {thinking_type}")

    return True
```

### 4. Permission Control

```python
# For tool-calling scenarios, validate permissions
def check_tool_permissions(tool_name: str, user_role: str):
    READONLY_TOOLS = {"file_read", "file_search", "lsp_check"}
    MUTATION_TOOLS = {"file_write", "splice_patch", "file_edit"}

    if user_role == "readonly" and tool_name in MUTATION_TOOLS:
        raise PermissionError(f"User cannot call {tool_name}")

    return True
```

---

## Token Usage and Budgeting

Deep Thinking consumes tokens for both reasoning and response:

```python
usage = response.usage
print(f"Prompt tokens: {usage.prompt_tokens}")
print(f"Completion tokens: {usage.completion_tokens}")
print(f"Total tokens: {usage.total_tokens}")

# Thinking-specific token count
thinking_tokens = response.choices[0].message.thinking_tokens
print(f"Thinking tokens: {thinking_tokens}")
```

**Budgeting recommendation**:
- Estimate: 1.5-2x normal token usage for Deep Thinking
- Reserve additional tokens for `reasoning_content`
- Monitor `thinking_tokens` separately for cost analysis

---

## Comparison: Standard vs. Deep Thinking

| Feature | Standard GLM-4 | GLM-4 Thinking |
|---------|---------------|----------------|
| Reasoning visibility | Hidden | Exposed |
| Token usage | Baseline | 1.5-2x baseline |
| Latency | Lower | Higher (more generation) |
| Use case | Simple queries | Complex reasoning |
| Audit trail | Limited | Full reasoning visible |

---

## OdinCode Integration Considerations

### Potential Uses

1. **Code refactoring reasoning**
   ```python
   thinking={
       "type": "preserved"
   }
   # Returns: Why symbols were selected, why transformation is safe
   ```

2. **Debugging tool failures**
   ```python
   # If tool call fails, preserve reasoning for analysis
   reasoning = response.choices[0].message.reasoning_content
   log_failure("tool_call", reasoning=reasoning)
   ```

3. **PromptMode explanations**
   ```python
   # LLM explains why it chose Query/Explore/Mutation mode
   # Useful for debugging mode classifier
   ```

### Implementation Notes

- **Model-agnostic design**: OdinCode's current design supports any model
- **Configuration**: Add `thinking_type` to `LlmConfig` if using GLM
- **Cost monitoring**: Track `thinking_tokens` separately in execution log

### Draft Configuration

```rust
// Potential future addition to src/llm/types.rs
#[derive(Debug, Clone, Copy, Default)]
pub enum ThinkingType {
    #[default]
    None,
    Interleaved,
    Preserved,
    TurnLevel,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub model: String,
    pub api_key: String,
    pub thinking_type: ThinkingType,  // For GLM
    // ... other fields
}
```

---

## API Endpoints

### Base URL

```
https://open.bigmodel.cn/api/paas/v4/
```

### Chat Completions Endpoint

```
POST https://open.bigmodel.cn/api/paas/v4/chat/completions
```

### Authentication

```python
# JWT-based API key generation
from zhipuai import ZhipuAI

client = ZhipuAI(api_key="your-api-key")
```

---

## Limitations

1. **Model-specific**: Only `glm-4-thinking` supports Deep Thinking
2. **Token overhead**: 1.5-2x token usage vs. standard completion
3. **Latency**: Additional generation time for thinking content
4. **Cost**: Higher per-query cost due to token usage

---

## References

- [Z.AI Deep Thinking Documentation](https://open.bigmodel.cn/dev/api#deep_thinking)
- [Z.AI Function Calling Guide](https://open.bigmodel.cn/dev/api#function_calling)
- [Z.AI API Reference](https://open.bigmodel.cn/dev/api)
- [Z.AI Quick Start](https://open.bigmodel.cn/dev/quickstart)

---

---

## Capability 2: Streaming Messages (SSE)

### Overview

Streaming uses Server-Sent Events (SSE) for real-time incremental content delivery during generation.

### Core Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `stream` | boolean | Yes | Set to `true` to enable streaming |
| `model` | string | Yes | `glm-4.7`, `glm-4.6`, `glm-4.5` |

### Response Format (SSE)

Each SSE chunk contains:

```python
for chunk in response:
    delta = chunk.choices[0].delta

    # Incremental content
    if hasattr(delta, 'content') and delta.content:
        full_content += delta.content

    # Incremental reasoning (Deep Thinking models)
    if hasattr(delta, 'reasoning_content') and delta.reasoning_content:
        full_reasoning += delta.reasoning_content

    # Completion status (last chunk only)
    if chunk.choices[0].finish_reason:
        print(f"Done: {chunk.choices[0].finish_reason}")
        print(f"Usage: {chunk.usage.prompt_tokens} + {chunk.usage.completion_tokens}")
```

### SSE Response Structure

```
data: {"id":"1","created":1677652288,"model":"glm-4.7","choices":[{"index":0,"delta":{"content":"Spring"},"finish_reason":null}]}

data: {"id":"1","created":1677652288,"model":"glm-4.7","choices":[{"index":0,"delta":{"content":" comes"},"finish_reason":null}]}

...

data: {"id":"1","created":1677652288,"model":"glm-4.7","choices":[{"finish_reason":"stop"}],"usage":{"prompt_tokens":8,"completion_tokens":262}}

data: [DONE]
```

### Python SDK Example

```python
from zai import ZaiClient

client = ZaiClient(api_key='Your API Key')

response = client.chat.completions.create(
    model="glm-4.7",
    messages=[{"role": "user", "content": "Write a poem about spring"}],
    stream=True  # Enable streaming
)

full_content = ""
for chunk in response:
    if not chunk.choices:
        continue

    delta = chunk.choices[0].delta

    if hasattr(delta, 'content') and delta.content:
        full_content += delta.content
        print(delta.content, end="", flush=True)

    if chunk.choices[0].finish_reason:
        print(f"\n\nReason: {chunk.choices[0].finish_reason}")
```

### Benefits

- **Real-time response**: No waiting for complete generation
- **Reduced perceived latency**: Content displays as it generates
- **Lower TTFB**: First chunk arrives quickly
- **Flexible processing**: Can process/analyze during reception

---

## Capability 3: Structured Output (JSON Mode)

### Overview

Ensures AI returns JSON conforming to predefined formats — **critical for programmatic processing**.

### Core Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `response_format` | object | Yes | `{"type": "json_object"}` to enable JSON mode |
| `model` | string | Yes | `glm-4.7`, `glm-4.6`, `glm-4.5` |

### Basic Usage

```python
from zai import ZaiClient
import json

client = ZaiClient(api_key="your-api-key")

response = client.chat.completions.create(
    model="glm-4.7",
    messages=[{
        "role": "system",
        "content": """
        You are a sentiment analysis expert. Return results in this JSON format:
        {
            "sentiment": "positive/negative/neutral",
            "confidence": 0.95,
            "emotions": ["joy", "excitement"],
            "keywords": ["weather", "mood"],
            "analysis": "Detailed explanation"
        }
        """
    }, {
        "role": "user",
        "content": "Analyze: 'The weather is nice today, I'm feeling happy!'"
    }],
    response_format={"type": "json_object"}
)

result = json.loads(response.choices[0].message.content)
print(f"Sentiment: {result['sentiment']}")
print(f"Confidence: {result['confidence']}")
```

### Schema Validation Pattern

```python
import jsonschema
from jsonschema import validate

schema = {
    "type": "object",
    "properties": {
        "sentiment": {
            "type": "string",
            "enum": ["positive", "negative", "neutral"]
        },
        "confidence": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
        },
        "emotions": {
            "type": "array",
            "items": {"type": "string"}
        }
    },
    "required": ["sentiment", "confidence"]
}

response = client.chat.completions.create(
    model="glm-4.7",
    messages=[...],
    response_format={"type": "json_object"}
)

result = json.loads(response.choices[0].message.content)
validate(instance=result, schema=schema)  # Throws if invalid
```

### Best Practices for Schema Design

1. **Clarity**: Field names and types should be explicit
2. **Completeness**: Include all validation rules (min/max, enum)
3. **Examples**: Provide example values in descriptions
4. **Nesting**: Keep structure shallow (< 4 levels deep)

---

## Capability 4: Context Caching (Automatic)

### Overview

**Automatic token optimization** — caches repeated context content (system prompts, long documents) without manual configuration.

### Features

| Feature | Description |
|---------|-------------|
| **Automatic recognition** | Implicit caching, no manual setup |
| **Cost reduction** | Cached tokens billed at ~50% of standard price |
| **Faster responses** | Reuses computation, reduces processing time |
| **Transparent billing** | `usage.prompt_tokens_details.cached_tokens` shows cache hits |

### How It Works

```
Request 1 (full price):
  System prompt: 500 tokens → $0.005
  User message: 50 tokens → $0.0005
  Response: 200 tokens → $0.002
  Total: $0.0075

Request 2 (same system prompt):
  System prompt: 500 tokens → CACHED → $0.0025 (50% off)
  User message: 50 tokens → $0.0005
  Response: 200 tokens → $0.002
  Total: $0.005 (33% savings)
```

### Cache Monitoring

```python
from zai import ZaiClient

client = ZaiClient(api_key='Your API Key')

# First request - establishes cache
response1 = client.chat.completions.create(
    model="glm-4.7",
    messages=[{
        "role": "system",
        "content": "You are a technical documentation assistant..."
    }, {
        "role": "user",
        "content": "What is REST?"
    }]
)

print(f"First request:")
print(f"  Total tokens: {response1.usage.total_tokens}")
print(f"  Cached tokens: {response1.usage.prompt_tokens_details.cached_tokens if hasattr(response1.usage, 'prompt_tokens_details') else 0}")

# Second request - reuses system prompt cache
response2 = client.chat.completions.create(
    model="glm-4.7",
    messages=[{
        "role": "system",
        "content": "You are a technical documentation assistant..."  # Same
    }, {
        "role": "user",
        "content": "What is GraphQL?"
    }]
)

print(f"\nSecond request:")
print(f"  Total tokens: {response2.usage.total_tokens}")
cached = response2.usage.prompt_tokens_details.cached_tokens if hasattr(response2.usage, 'prompt_tokens_details') else 0
print(f"  Cached tokens: {cached}")
print(f"  Cache savings: {cached / response2.usage.total_tokens * 100:.1f}%")
```

### Best Practices

1. **Use stable system prompts** — Don't include variable content in system messages
2. **Long documents as system messages** — Put document content in system role for caching
3. **Monitor cache hit rates** — Track `cached_tokens` to optimize usage patterns

### Cache-Effective Pattern

```python
# ✅ Good: Document in system message (cached across queries)
def analyze_document(document: str, questions: list):
    system_prompt = f"Answer questions based on this document:\n{document}"

    results = []
    for question in questions:
        response = client.chat.completions.create(
            model="glm-4.7",
            messages=[
                {"role": "system", "content": system_prompt},  # Cached
                {"role": "user", "content": question}
            ]
        )
        results.append(response.choices[0].message.content)

    return results

# ❌ Bad: Document in user message (not cached)
def analyze_document_bad(document: str, questions: list):
    results = []
    for question in questions:
        response = client.chat.completions.create(
            model="glm-4.7",
            messages=[
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": f"Document: {document}\nQuestion: {question}"}  # Not cached
            ]
        )
        results.append(response.choices[0].message.content)

    return results
```

---

## Capability 5: Function Calling

### Overview

AI models can **call external functions** to interact with systems beyond their training data.

### Core Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `tools` | array | List of callable functions (name, description, parameters) |
| `tool_choice` | string | `"auto"` (default) — model decides whether to call |
| `model` | string | `glm-4-plus`, `glm-4.7`, `glm-4.6` |

### Function Definition Schema

```python
tools = [{
    "type": "function",
    "function": {
        "name": "get_weather",
        "description": "Get current weather for a city",
        "parameters": {
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "City name, e.g.: Beijing, Shanghai"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit"
                }
            },
            "required": ["city"]
        }
    }
}]
```

### Complete Workflow

```python
import json
from zai import ZaiClient

client = ZaiClient(api_key='your_api_key')

# 1. Define the actual function
def get_weather(city: str, unit: str = "celsius") -> dict:
    # Call real weather API here
    return {
        "city": city,
        "temperature": "22°C" if unit == "celsius" else "72°F",
        "condition": "Sunny",
        "humidity": "65%"
    }

# 2. Define tools for the model
tools = [{
    "type": "function",
    "function": {
        "name": "get_weather",
        "description": "Get current weather for specified city",
        "parameters": {
            "type": "object",
            "properties": {
                "city": {"type": "string", "description": "City name"},
                "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}
            },
            "required": ["city"]
        }
    }
}]

# 3. Make request
response = client.chat.completions.create(
    model="glm-4.7",
    messages=[{"role": "user", "content": "How's the weather in Beijing?"}],
    tools=tools,
    tool_choice="auto"
)

# 4. Handle tool calls
message = response.choices[0].message
messages = [{"role": "user", "content": "How's the weather in Beijing?"}]
messages.append(message.model_dump())

if message.tool_calls:
    for tool_call in message.tool_calls:
        if tool_call.function.name == "get_weather":
            # Parse arguments and call function
            args = json.loads(tool_call.function.arguments)
            result = get_weather(args["city"], args.get("unit", "celsius"))

            # Return result to model
            messages.append({
                "role": "tool",
                "content": json.dumps(result, ensure_ascii=False),
                "tool_call_id": tool_call.id
            })

    # Get final answer
    final_response = client.chat.completions.create(
        model="glm-4.7",
        messages=messages,
        tools=tools
    )

    print(final_response.choices[0].message.content)
```

### Best Practices

**Parameter Design**:
```python
# ✅ Good: Detailed description with examples
{
    "city": {
        "type": "string",
        "description": "City name, supports Chinese and English, e.g.: Beijing, Shanghai, New York",
        "examples": ["Beijing", "Shanghai", "New York"]
    }
}

# ❌ Bad: Vague description
{
    "city": {
        "type": "string",
        "description": "City"
    }
}
```

**Error Handling**:
```python
def robust_function(param: str) -> dict:
    try:
        if not param or not isinstance(param, str):
            return {
                "success": False,
                "error": "Invalid parameter",
                "error_code": "INVALID_PARAM"
            }

        result = process_data(param)
        return {"success": True, "data": result}

    except ValueError as e:
        return {"success": False, "error": f"Data error: {e}", "error_code": "DATA_ERROR"}
    except Exception as e:
        return {"success": False, "error": f"System error: {e}", "error_code": "SYSTEM_ERROR"}
```

**Input Validation**:
```python
def secure_function(user_input: str) -> dict:
    # Length limit
    if len(user_input) > 1000:
        return {"error": "Input too long"}

    # Dangerous character filtering
    dangerous_chars = ['<', '>', '&', '"', "'"]
    if any(char in user_input for char in dangerous_chars):
        return {"error": "Input contains dangerous characters"}

    return {"success": True, "processed": user_input}
```

---

## Capability 6: Tool Streaming Output (GLM-4.6 Only)

### Overview

**Stream tool calls** in real-time — parameters stream incrementally without buffering. GLM-4.6 exclusive.

### Core Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `stream` | boolean | Yes | `true` for streaming |
| `tool_stream` | boolean | Yes | `true` for tool streaming |
| `model` | string | Yes | `glm-4.6` only |

### Response Delta Fields

```python
for chunk in response:
    delta = chunk.choices[0].delta

    # Reasoning process (if Deep Thinking enabled)
    if hasattr(delta, 'reasoning_content') and delta.reasoning_content:
        reasoning += delta.reasoning_content

    # Response content
    if hasattr(delta, 'content') and delta.content:
        content += delta.content

    # Tool calls (streaming!)
    if delta.tool_calls:
        for tool_call in delta.tool_calls:
            index = tool_call.index
            if index not in final_tool_calls:
                final_tool_calls[index] = tool_call
                final_tool_calls[index].function.arguments = tool_call.function.arguments
            else:
                # Append streaming arguments
                final_tool_calls[index].function.arguments += tool_call.function.arguments
```

### Complete Example

```python
from zai import ZaiClient

client = ZaiClient(api_key='Your API Key')

response = client.chat.completions.create(
    model="glm-4.6",  # Only model supporting tool streaming
    messages=[{"role": "user", "content": "How's the weather in Beijing?"}],
    tools=[{
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather for a city",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {"type": "string"},
                    "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}
                },
                "required": ["location"]
            }
        }
    }],
    stream=True,       # Enable streaming
    tool_stream=True    # Enable tool streaming
)

reasoning_content = ""
content = ""
final_tool_calls = {}

for chunk in response:
    if not chunk.choices:
        continue

    delta = chunk.choices[0].delta

    # Handle reasoning
    if hasattr(delta, 'reasoning_content') and delta.reasoning_content:
        reasoning_content += delta.reasoning_content
        print(delta.reasoning_content, end="", flush=True)

    # Handle content
    if hasattr(delta, 'content') and delta.content:
        content += delta.content
        print(delta.content, end="", flush=True)

    # Handle tool calls
    if delta.tool_calls:
        for tool_call in delta.tool_calls:
            index = tool_call.index
            if index not in final_tool_calls:
                final_tool_calls[index] = tool_call
                final_tool_calls[index].function.arguments = tool_call.function.arguments
            else:
                final_tool_calls[index].function.arguments += tool_call.function.arguments

if final_tool_calls:
    print("\n\nTool Calls:")
    for index, tool_call in final_tool_calls.items():
        print(f"  {tool_call.function.name}({tool_call.function.arguments})")
```

### Use Cases

- **Real-time tool monitoring**: See tool choice as soon as model decides
- **Reduced latency**: Start tool execution before full response completes
- **Progressive UI**: Show "calling tool..." indicator immediately

---

## Model Comparison

| Model | Deep Thinking | Streaming | Structured Output | Context Cache | Function Call | Tool Streaming |
|-------|--------------|-----------|-------------------|---------------|---------------|----------------|
| `glm-4-thinking` | ✅ | ✅ | ❓ | ✅ | ✅ | ❌ |
| `glm-4.7` | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `glm-4.6` | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `glm-4.5` | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `glm-4.5-air` | Hybrid | ✅ | ✅ | ✅ | ✅ | ❌ |
| `glm-4-plus` | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ |

---

## API Endpoints

### Base URL

```
https://api.z.ai/api/paas/v4/    # International (docs.z.ai)
https://open.bigmodel.cn/api/paas/v4/  # China (open.bigmodel.cn)
```

### Chat Completions

```
POST /api/paas/v4/chat/completions
```

### Headers

```http
Authorization: Bearer YOUR_API_KEY
Content-Type: application/json
```

---

## SDK Installation

```bash
# Python (Z.AI SDK)
pip install zai-sdk

# Or specify version
pip install zai-sdk==0.1.0
```

```python
import zai
print(zai.__version__)  # Verify installation
```

---

## Summary

| Capability | Primary Parameter | Key Benefit | OdinCode Relevance |
|------------|-------------------|-------------|-------------------|
| **Deep Thinking** | `thinking.type` | Exposed reasoning | Debug mode selection, audit decisions |
| **Streaming** | `stream=true` | Real-time output | Progressive UI for long operations |
| **Structured Output** | `response_format={"type": "json_object"}` | Guaranteed JSON | Reliable tool response parsing |
| **Context Caching** | Automatic | ~50% cost savings | System prompt reuse across sessions |
| **Function Calling** | `tools`, `tool_choice` | External system access | Core substrate integration |
| **Tool Streaming** | `tool_stream=true` | Real-time tool calls | Faster tool execution feedback |

### OdinCode Integration Notes

1. **Reasoning to database**: Store `reasoning_content` in execution_log for debugging mode classification
2. **Structured output for tools**: Use JSON mode for tool response parsing reliability
3. **Context caching**: FrameStack already implements similar caching — align with GLM's automatic approach
4. **Function calling**: Our TOOL_WHITELIST maps directly to GLM's `tools` array
