use crate::types::*;
use anyhow::{Context, Result};
use octocrab::{Octocrab, models::pulls::PullRequest};
use tracing::{debug, info};

/// GitHub API client for PR analysis
pub struct GitHubClient {
    client: Octocrab,
}

impl GitHubClient {
    /// Create a new GitHub client with authentication token
    pub fn new(token: String) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token)
            .build()
            .context("Failed to create GitHub client")?;

        Ok(Self { client })
    }

    /// Get PR context including changed files and metadata
    pub async fn get_pr_context(&self, repo: &str, pr_number: u64) -> Result<PRContext> {
        info!("Fetching PR context for {}/pull/{}", repo, pr_number);

        let (owner, repo_name) = self.parse_repo(repo)?;
        
        // Get PR details
        let pr = self.client
            .pulls(&owner, &repo_name)
            .get(pr_number)
            .await
            .context("Failed to fetch PR details")?;

        // Get PR files
        let files = self.client
            .pulls(&owner, &repo_name)
            .list_files(pr_number)
            .send()
            .await
            .context("Failed to fetch PR files")?;

        // Convert files to ChangeDetail
        let mut changed_files = Vec::new();
        for file in files {
            let status = match file.status.as_str() {
                "added" => ChangeStatus::Added,
                "modified" => ChangeStatus::Modified,
                "removed" => ChangeStatus::Deleted,
                "renamed" => ChangeStatus::Renamed,
                _ => ChangeStatus::Modified,
            };

            changed_files.push(ChangeDetail {
                filename: file.filename,
                status,
                additions: file.additions,
                deletions: file.deletions,
                patch: file.patch,
            });
        }

        debug!("Found {} changed files in PR", changed_files.len());

        Ok(PRContext {
            repo_name: repo.to_string(),
            pr_number,
            base_branch: pr.base.ref_field,
            head_branch: pr.head.ref_field,
            changed_files,
            author: pr.user.login,
            title: pr.title.unwrap_or_default(),
            description: pr.body,
            created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
        })
    }

    /// Post a comment on the PR
    pub async fn post_comment(&self, repo: &str, pr_number: u64, body: &str) -> Result<()> {
        let (owner, repo_name) = self.parse_repo(repo)?;

        self.client
            .issues(&owner, &repo_name)
            .create_comment(pr_number, body)
            .await
            .context("Failed to post PR comment")?;

        info!("Posted comment on PR #{}", pr_number);
        Ok(())
    }

    /// Update PR status check
    pub async fn update_status_check(
        &self,
        repo: &str,
        commit_sha: &str,
        context: &str,
        state: &str,
        description: &str,
    ) -> Result<()> {
        let (owner, repo_name) = self.parse_repo(repo)?;

        let state = match state {
            "success" => octocrab::models::StatusState::Success,
            "failure" => octocrab::models::StatusState::Failure,
            "error" => octocrab::models::StatusState::Error,
            _ => octocrab::models::StatusState::Pending,
        };

        self.client
            .repos(&owner, &repo_name)
            .create_status(commit_sha, state)
            .context(context)
            .description(description)
            .send()
            .await
            .context("Failed to update status check")?;

        info!("Updated status check for commit {}", commit_sha);
        Ok(())
    }

    /// Parse repository string into owner and name
    fn parse_repo(&self, repo: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid repository format. Expected 'owner/repo', got '{}'", repo));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Check if the client can authenticate
    pub async fn check_authentication(&self) -> Result<String> {
        let user = self.client
            .current()
            .user()
            .await
            .context("Failed to authenticate with GitHub")?;

        Ok(user.login)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo() {
        let client = GitHubClient { 
            client: Octocrab::builder().build().unwrap() 
        };
        
        let (owner, repo) = client.parse_repo("owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
        
        assert!(client.parse_repo("invalid").is_err());
        assert!(client.parse_repo("too/many/parts").is_err());
    }
}