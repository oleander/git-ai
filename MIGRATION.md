# Migration Guide

## Model Changes (v1.0.9+)

### Overview
Git AI now supports GPT-4.1 family models exclusively. The previous models (GPT-4, GPT-4o, GPT-4o-mini) are deprecated but will continue to work with automatic mapping to equivalent GPT-4.1 models.

### Supported Models

| Model Name | Description | Use Case |
|------------|-------------|----------|
| `gpt-4.1` | Default model | General purpose, best quality (default) |
| `gpt-4.1-mini` | Faster processing | Balanced speed and quality |
| `gpt-4.1-nano` | Ultra-fast | Maximum speed |
| `gpt-4.5` | Advanced model | Complex commits requiring more context |

### Deprecated Models (Backward Compatible)

The following models are deprecated but will automatically map to GPT-4.1 equivalents:

| Deprecated Model | Maps To | Warning Level |
|------------------|---------|---------------|
| `gpt-4` | `gpt-4.1` | Deprecation warning logged |
| `gpt-4o` | `gpt-4.1` | Deprecation warning logged |
| `gpt-4o-mini` | `gpt-4.1-mini` | Deprecation warning logged |
| `gpt-3.5-turbo` | `gpt-4.1-mini` | Deprecation warning logged |

### Migration Steps

#### 1. Check Your Current Model

```bash
cat ~/.config/git-ai/config.ini | grep model
```

#### 2. Update to a Supported Model

If you're using a deprecated model, update your configuration:

```bash
# For best quality (default)
git ai config set model gpt-4.1

# For balanced speed and quality
git ai config set model gpt-4.1-mini

# For maximum speed
git ai config set model gpt-4.1-nano

# For complex commits
git ai config set model gpt-4.5
```

### Behavior Changes

#### Before
```bash
$ git ai config set model gpt-4o
✅ Model set to: gpt-4o
$ git commit --no-edit
# Uses gpt-4o directly
```

#### After
```bash
$ git ai config set model gpt-4o
✅ Model set to: gpt-4o
$ git commit --no-edit
⚠️  WARN: Model 'gpt-4o' is deprecated. Mapping to 'gpt-4.1'.
    Please update your configuration with: git ai config set model gpt-4.1
# Commit proceeds using gpt-4.1
```

### Invalid Model Names

If you configure an invalid model name:

1. **Configuration**: Accepts any string (for forward compatibility)
2. **Runtime**: Logs an error and falls back to `gpt-4.1` (default)
3. **OpenAI API Error**: If OpenAI doesn't recognize the model, the error is logged before fallback

```bash
$ git ai config set model does-not-exist
✅ Model set to: does-not-exist

$ git commit --no-edit
❌ ERROR: Failed to parse model 'does-not-exist': Invalid model name: 'does-not-exist'.
    Falling back to default model 'gpt-4.1'.
# Commit proceeds using gpt-4.1
```

### Testing Your Configuration

Test your model configuration without making a commit:

```bash
# Set test mode
export RUST_LOG=debug

# Make a dummy change
echo "test" >> test.txt
git add test.txt

# Try committing (you can abort if needed)
git commit --no-edit

# Check the logs for model information
# Should show: "Using model: gpt-4.1, Tokens: X, ..."
```

### Rollback

If you need to rollback to the previous version:

```bash
# Uninstall current version
cargo uninstall git-ai

# Install specific older version (before model changes)
cargo install git-ai --version 1.0.8
```

### Questions?

- **Q: Will my old model configuration stop working?**
  A: No, deprecated models automatically map to GPT-4.1 equivalents with a warning.

- **Q: Why the change to GPT-4.1 models?**
  A: GPT-4.1 models offer better performance, lower latency, and reduced costs while matching or exceeding the quality of previous models.

- **Q: What if I want to use a model not in the list?**
  A: Currently only the 4 GPT-4.1 family models are supported. Custom models will fall back to the default with an error logged.

- **Q: Can I still use GPT-4o?**
  A: Yes, but it will automatically map to GPT-4.1. Update your config to avoid deprecation warnings.
