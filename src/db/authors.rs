//! Author profile cache and alias lookup.

use super::*;

impl Catalog {
    pub fn get_author_profile_by_name(&self, name: &str) -> Result<Option<AuthorProfile>> {
        let key = crate::author::normalize_author_name(name);
        if key.is_empty() {
            return Ok(None);
        }
        let conn = self.conn();
        let id = conn
            .query_row(
                "SELECT id FROM author_profiles WHERE normalized_name = ?1
                 UNION
                 SELECT author_id FROM author_aliases WHERE normalized_alias = ?1
                 LIMIT 1",
                params![key],
                |r| r.get::<_, i64>(0),
            )
            .optional()?;
        let Some(id) = id else {
            return Ok(None);
        };
        load_author_profile(&conn, id)
    }

    pub fn upsert_author_profile(&self, profile: &AuthorProfile) -> Result<i64> {
        let conn = self.conn();
        let fetched_at = chrono_like_now();
        let subjects_json =
            serde_json::to_string(&profile.top_subjects).unwrap_or_else(|_| "[]".into());
        let works_json = serde_json::to_string(&profile.works).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT INTO author_profiles
                (canonical_name, sort_name, normalized_name, bio, birth_date, death_date,
                 top_work, top_subjects_json, openlibrary_key, photo_file, work_count,
                 works_json, fetched_at, source_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(normalized_name) DO UPDATE SET
                 canonical_name = excluded.canonical_name,
                 sort_name = excluded.sort_name,
                 bio = excluded.bio,
                 birth_date = excluded.birth_date,
                 death_date = excluded.death_date,
                 top_work = excluded.top_work,
                 top_subjects_json = excluded.top_subjects_json,
                 openlibrary_key = excluded.openlibrary_key,
                 photo_file = COALESCE(excluded.photo_file, author_profiles.photo_file),
                 work_count = excluded.work_count,
                 works_json = excluded.works_json,
                 fetched_at = excluded.fetched_at,
                 source_url = excluded.source_url",
            params![
                profile.canonical_name.trim(),
                profile.sort_name.trim(),
                if profile.normalized_name.trim().is_empty() {
                    crate::author::normalize_author_name(&profile.canonical_name)
                } else {
                    profile.normalized_name.clone()
                },
                profile.bio.trim(),
                profile.birth_date.trim(),
                profile.death_date.trim(),
                profile.top_work.trim(),
                subjects_json,
                profile.openlibrary_key.trim(),
                profile.photo_file.as_deref(),
                profile.work_count,
                works_json,
                fetched_at,
                profile.source_url.trim(),
            ],
        )?;

        let author_id: i64 = conn.query_row(
            "SELECT id FROM author_profiles WHERE normalized_name = ?1",
            params![if profile.normalized_name.trim().is_empty() {
                crate::author::normalize_author_name(&profile.canonical_name)
            } else {
                profile.normalized_name.clone()
            }],
            |r| r.get(0),
        )?;

        let mut aliases = profile.aliases.clone();
        aliases.push(profile.canonical_name.clone());
        aliases.push(profile.sort_name.clone());
        for alias in aliases {
            let alias = alias.trim();
            if alias.is_empty() {
                continue;
            }
            let normalized = crate::author::normalize_author_name(alias);
            if normalized.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT INTO author_aliases (author_id, alias, normalized_alias)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(normalized_alias) DO UPDATE SET
                     author_id = excluded.author_id,
                     alias = excluded.alias",
                params![author_id, alias, normalized],
            )?;
        }
        Ok(author_id)
    }
}

fn load_author_profile(conn: &Connection, id: i64) -> Result<Option<AuthorProfile>> {
    let row = conn
        .query_row(
            "SELECT id, canonical_name, sort_name, normalized_name, bio, birth_date,
                    death_date, top_work, top_subjects_json, openlibrary_key, photo_file,
                    work_count, works_json, fetched_at, source_url
             FROM author_profiles WHERE id = ?1",
            params![id],
            |r| {
                let photo_file: Option<String> = r.get(10)?;
                let top_subjects = json_vec_string(&r.get::<_, String>(8)?);
                let works = json_author_works(&r.get::<_, String>(12)?);
                Ok(AuthorProfile {
                    id: r.get(0)?,
                    canonical_name: r.get(1)?,
                    sort_name: r.get(2)?,
                    normalized_name: r.get(3)?,
                    bio: r.get(4)?,
                    birth_date: r.get(5)?,
                    death_date: r.get(6)?,
                    top_work: r.get(7)?,
                    top_subjects,
                    openlibrary_key: r.get(9)?,
                    photo_path: photo_file
                        .as_ref()
                        .map(|file| crate::paths::authors_dir().join(file)),
                    photo_file,
                    work_count: r.get(11)?,
                    works,
                    aliases: Vec::new(),
                    fetched_at: r.get(13)?,
                    source_url: r.get(14)?,
                })
            },
        )
        .optional()?;
    let Some(mut profile) = row else {
        return Ok(None);
    };

    let mut stmt = conn.prepare_cached(
        "SELECT alias FROM author_aliases WHERE author_id = ?1 ORDER BY alias COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
    profile.aliases = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Some(profile))
}

fn json_vec_string(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn json_author_works(raw: &str) -> Vec<AuthorWork> {
    serde_json::from_str(raw).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_author_by_display_or_sort_name() {
        let catalog = Catalog::open_in_memory().unwrap();
        let profile = AuthorProfile {
            canonical_name: "George R. R. Martin".into(),
            sort_name: "Martin, George R. R.".into(),
            normalized_name: crate::author::normalize_author_name("George R. R. Martin"),
            aliases: vec!["George R.R. Martin".into()],
            ..AuthorProfile::default()
        };
        catalog.upsert_author_profile(&profile).unwrap();

        let by_display = catalog
            .get_author_profile_by_name("George R. R. Martin")
            .unwrap()
            .unwrap();
        let by_sort = catalog
            .get_author_profile_by_name("Martin, George R. R.")
            .unwrap()
            .unwrap();

        assert_eq!(by_display.canonical_name, "George R. R. Martin");
        assert_eq!(by_sort.canonical_name, "George R. R. Martin");
    }
}
