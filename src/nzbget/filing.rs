use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Filed, Filing};

use super::settings::{Entry, Settings};
use super::Nzbget;

const CATEGORY: &str = "Category";

const NAME: &str = "Name";

const DEST_DIR: &str = "DestDir";

const APPEND_CATEGORY_DIR: &str = "AppendCategoryDir";

const YES: &str = "yes";

const VARIABLE: &str = "${";

#[async_trait]
impl Filing for Nzbget {
    async fn filed(&self) -> Result<Vec<Filed>, Failure> {
        let running = self.live("the categories could not be read").await?;
        Ok(filed_in(&running))
    }

    async fn file_under(&self, folders: &[Filed]) -> Result<(), Failure> {
        if let Some(misshapen) = folders.iter().find(|one| !held_to_shape(one)) {
            return Err(self.endpoint.refused(&format!(
                "{:?} under {:?} is not a category and folder NZBGet could hold",
                misshapen.category, misshapen.folder
            )));
        }
        let running = self.live("the categories could not be read").await?;
        if holds(&running, folders) {
            return Ok(());
        }
        let mut next = self.saved("the categories could not be read").await?;
        let mut categories = next.numbered(CATEGORY);
        for one in folders {
            match categories
                .iter_mut()
                .find(|category| category.get(NAME).eq_ignore_ascii_case(&one.category))
            {
                Some(category) => category.set(DEST_DIR, &one.folder),
                None => {
                    categories.push(Entry::of(&[(NAME, &one.category), (DEST_DIR, &one.folder)]));
                }
            }
        }
        next.renumber(CATEGORY, &categories);
        self.reloaded(next, true, &|running| holds(running, folders))
            .await?;
        Ok(())
    }
}

fn filed_in(settings: &Settings) -> Vec<Filed> {
    let below = settings.get(DEST_DIR).trim_end_matches('/');
    let appended = settings.get(APPEND_CATEGORY_DIR).eq_ignore_ascii_case(YES);
    settings
        .numbered(CATEGORY)
        .iter()
        .filter(|category| !category.get(NAME).trim().is_empty())
        .map(|category| {
            let name = category.get(NAME).trim();
            let own = category.get(DEST_DIR).trim();
            let folder = if !own.is_empty() {
                own.to_owned()
            } else if appended {
                format!("{below}/{name}")
            } else {
                below.to_owned()
            };
            Filed {
                category: name.to_owned(),
                folder,
            }
        })
        .collect()
}

fn holds(settings: &Settings, folders: &[Filed]) -> bool {
    let filed = filed_in(settings);
    folders.iter().all(|one| {
        filed.iter().any(|held| {
            held.category.eq_ignore_ascii_case(&one.category) && held.folder == one.folder
        })
    })
}

fn held_to_shape(one: &Filed) -> bool {
    let category = one.category.as_str();
    let named = !category.trim().is_empty()
        && category
            .chars()
            .all(|letter| letter.is_alphanumeric() || matches!(letter, '-' | '_' | '.' | ' '));
    let placed = one.folder.starts_with('/')
        && !one.folder.contains(VARIABLE)
        && !one.folder.chars().any(char::is_control)
        && !one.folder.split('/').any(|segment| segment == "..");
    named && placed
}

#[cfg(test)]
mod tests;
