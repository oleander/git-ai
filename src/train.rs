use llm_chain::chains::map_reduce::Chain;
use llm_chain::step::Step;
use llm_chain::{executor, parameters, prompt, Parameters};
use git2::Repository;

fn get_last_n_commits(repo_path: &str, n: usize) -> Vec<String> {
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let mut revwalk = repo.revwalk().expect("Failed to create revwalk");
    revwalk.push_head().expect("Failed to push head");
    revwalk.take(n).map(|id| {
        let id = id.expect("Failed to get commit id");
        let commit = repo.find_commit(id).expect("Failed to find commit");
        commit.summary().unwrap_or_default().to_string()
    }).collect()
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exec = executor!()?;

    let map_prompt = Step::for_prompt_template(prompt!(
        "You are an AI trained to analyze code diffs and generate commit messages that match the style and tonality of previous commits.",
        "Given the context of the previous commit message: '{{last_commit_message}}', analyze this code diff: '{{code_diff}}', and suggest a new commit message that maintains a similar style and tone."
    ));

    let reduce_prompt = Step::for_prompt_template(prompt!(
        "You are an AI summarizing multiple code changes in the context of past commits for a comprehensive commit message.",
        "Combine these change analyses with the context of the last commit message: '{{last_commit_message}}' into a cohesive new commit message."
    ));

    let chain = Chain::new(map_prompt, reduce_prompt);
    let current_dir = std::env::current_dir().unwrap();
    let commits = get_last_n_commits(current_dir.to_str().unwrap(), 3);
    let docs = commits.iter().map(|msg| parameters!(msg)).collect::<Vec<_>>();
    let res = chain.run(docs, Parameters::new(), &exec).await?;

    println!("{}", res);
    Ok(())
}


#[tokio::main(flavor = "current_thread")]
async fn xmain() -> Result<(), Box<dyn std::error::Error>> {
    let exec = executor!()?;

    let map_prompt = Step::for_prompt_template(prompt!(
        "You are an AI trained to analyze code diffs and generate commit messages that match the style and tonality of previous commits.",
        "Given the context of the previous commit message: '{{last_commit_message}}', analyze this code diff: '{{code_diff}}', and suggest a new commit message that maintains a similar style and tone."
    ));

    let reduce_prompt = Step::for_prompt_template(prompt!(
        "You are an AI summarizing multiple code changes in the context of past commits for a comprehensive commit message.",
        "Combine these change analyses with the context of the last commit message: '{{last_commit_message}}' into a cohesive new commit message."
    ));

    let chain = Chain::new(map_prompt, reduce_prompt);

    let last_commit_message = "Your last commit message here";
    let code_diff = "Your code diff here";

    let docs = vec![parameters!(last_commit_message, code_diff)];

    let res = chain.run(docs, Parameters::new(), &exec).await?;

    println!("{}", res);
    Ok(())
}