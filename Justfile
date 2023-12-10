BIN := "target/release/git-ai"

simulation:
    @rm -rf /tmp/git-ai
    @mkdir /tmp/git-ai
    @cd /tmp/git-ai
    @git init
    @echo "Hello World" > README.md
    @git add README.md
    @git commit -m "Initial commit"
    @echo "Hello World!" >> README.md
    @git add README.md
    @$BIN install
    @git commit --no-edit
    @rm -rf /tmp/git-ai
