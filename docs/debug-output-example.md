# Git AI Debug Output Example

This document shows an example of the comprehensive debug output generated when `RUST_LOG=debug` is set.

## Example Debug Output

```
=== GIT AI HOOK DEBUG SESSION ===

📋 INITIALIZATION
  Args:        commit_msg_file='.git/COMMIT_EDITMSG', source=None, sha1=None
  Build:       Debug build with performance profiling enabled

⚙️  SETUP & PREPARATION
  │ Generate instruction template     1.56ms    ✓
  │ Count tokens                      306.13ms  ✓
  │ Calculate instruction tokens      307.77ms  ✓
  └ Get context size                 959.00ns  ✓

📝 GIT DIFF PROCESSING
  │ Git diff generation               3.66ms    ✓
  │ Processing diff changes           416.13µs  ✓
  │ Repository patch generation       804.25µs  ✓
  └ Files parsed from diff           1 files   ✓

🤖 AI PROCESSING
  Multi-Step Attempt:                           FAILED
    │ Creating score function tool              ✓
    │ OpenAI connection                         ✓
    └ Error: Invalid function_call             ✗ No function named 'required' specified

  Single-Step Fallback:                        SUCCESS
    │ Creating commit function tool             ✓ max_length=72
    │ OpenAI API call                   2.78s   ✓
    └ Response parsing                          ✓

📊 ANALYSIS RESULTS
  Commit Message: 'Use multi-step commit message generation by default with fallback'
  Message Length: 65 characters (within 72 limit)

  Reasoning:
    The diff shows that both generate_commit_message and call_with_config
    now try multi-step generation first, falling back to local or single-step
    only on error. This changes core commit message logic, making multi-step
    the default approach (highest impact in source file).

📁 FILE ANALYSIS
  Total Files: 1

  File Details:
    src/openai.rs (modified)
      │ Summary: Default to multi-step commit message generation with fallback
      │ Impact Score: 1.00 (highest)
      │ Lines Changed: 35
      └ Category: source

📈 STATISTICS SUMMARY
  │ Total Lines Changed:     35
  │ Average Impact Score:    1.00
  │
  │ By Category:
  │   └ source: 1
  │
  │ By Change Type:
  │   └ modified: 1

⏱️  PERFORMANCE SUMMARY
  │ OpenAI request/response:          2.78s
  │ Total execution time:             3.46s
  └ Status:                           SUCCESS ✓

🎯 FINAL RESULT
  [feature/function 7a18ca0] Use multi-step commit message generation by default with fallback
   1 file changed, 41 insertions(+), 3 deletions(-)
```

## Debug Output Sections Explained

### 1. Initialization
Shows the git hook arguments and build configuration.

### 2. Setup & Preparation
Tracks timing for:
- Template generation
- Token counting
- Context size calculation

### 3. Git Diff Processing
Shows:
- Diff generation timing
- File parsing results
- Number of files detected

### 4. AI Processing
Details the multi-step attempt and fallback:
- Multi-step errors (if any)
- Single-step fallback success
- API call duration

### 5. Analysis Results
Displays:
- Generated commit message
- Message length validation
- AI reasoning for the message

### 6. File Analysis
For each file:
- Path and operation type
- Summary of changes
- Impact score
- Lines changed
- File category

### 7. Statistics Summary
Aggregated data:
- Total lines changed
- Average impact score
- Files grouped by category
- Files grouped by change type

### 8. Performance Summary
Overall timing:
- API request/response time
- Total execution time
- Success status

### 9. Final Result
Shows the actual git commit result:
- Branch name
- Commit hash (short)
- Commit message
- File change statistics

## Enabling Debug Output

To see this debug output, run git commits with debug logging enabled:

```bash
RUST_LOG=debug git commit
```

Or set it globally for the session:

```bash
export RUST_LOG=debug
git commit
```

## Implementation Notes

The debug output is only shown when:
1. `RUST_LOG=debug` is set
2. The build is a debug build (or release with debug logging)
3. The git hook successfully processes the commit

The final result section would typically be populated by git itself after the hook completes, showing the actual commit hash and statistics.
