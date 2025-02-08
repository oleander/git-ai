# Finetune.rs Workflow

Here's a summary of the workflow in `finetune.rs`:

- Uses GPT4o-mini model for OpenAI
- Generates training data in JSONL format for fine-tuning
- Splits data into training and verification sets

1. **Initialize and Setup**

   - Creates empty train and verify files
   - Sets up thread pool for parallel processing
   - Initializes progress bars and counters
   - Loads system prompt from `resources/prompt.md`

2. **Collect Commit History**

   - Opens local git repository
   - Walks through commit history
   - Filters commits based on:
     - Message length (20-500 chars)
     - Non-merge commits only
     - Diff size within limits (default 5000 chars)
   - Collects valid commits up to 3x target number
   - Shuffles commits for randomization

3. **Process Commits in Parallel**

   - Spawns worker threads based on CPU count or user setting
   - Each worker processes a subset of commits
   - For each commit:
     - Checks for duplicate messages
     - Rates commit quality (0.0-1.0)
     - Cleans up commit message
     - Tracks approved commits with progress bar
     - Stops when target number reached

4. **Clean and Rate Commit Messages**

   - Cleanup process:
     - Takes first line only
     - Removes ticket references and tags
     - Ensures proper capitalization
     - Drops type prefixes
     - Keeps messages short and meaningful
   - Quality rating based on:
     - Message format and clarity
     - Diff alignment
     - Present tense and active voice
     - Description accuracy

5. **Generate Training Data**

   - Creates JSONL entries with:
     - System prompt
     - Diff as user input
     - Cleaned message as assistant output
   - Splits data:
     - 50% for training
     - 50% for verification
   - Prevents duplicate messages
   - Validates cleaned messages

6. **Track Progress and Results**
   - Shows real-time progress:
     - Commit collection progress
     - Message cleaning progress
     - Approval status
   - Reports final statistics:
     - Total commits processed
     - Training examples count
     - Verification examples count
     - Distribution between files

Key Features:

- Parallel processing for better performance
- Double quality check (original and cleaned messages)
- Duplicate prevention at multiple stages
- Progress visualization with spinners and bars
- Verbose mode for detailed logging

The key difference from optimize.rs is that finetune.rs focuses on generating high-quality training data for fine-tuning, while optimize.rs focuses on improving the system prompt itself.

Note: Run sync, not async
