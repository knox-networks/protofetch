use std::collections::HashMap;
use std::path::Path;
use std::str::Utf8Error;

use crate::cache::{CacheError, RepositoryCache};
use crate::model::{Coordinate, Dependency, LockFile, LockedDependency, Revision};
use crate::proto_repository::ProtoRepository;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Error while fetching repo from cache: {0}")]
    Cache(#[from] CacheError),
    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),
    #[error("Error while decoding utf8 bytes from blob: {0}")]
    BlobRead(#[from] Utf8Error),
    #[error("Error while parsing descriptor")]
    Parsing(#[from] crate::model::ParseError),
    #[error("Bad output dir {0}")]
    BadOutputDir(String),
    #[error("Error while processing protobuf repository: {0}")]
    ProtoRepoError(#[from] crate::proto_repository::ProtoRepoError),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
}

pub fn lock<Cache: RepositoryCache>(
    module_name: String,
    cache: &Cache,
    dependencies: &[Dependency],
) -> Result<LockFile, FetchError> {
    fn go<Cache: RepositoryCache>(
        cache: &Cache,
        dep_map: &mut HashMap<Coordinate, Vec<Revision>>,
        repo_map: &mut HashMap<Coordinate, (String, ProtoRepository)>,
        dependencies: &[Dependency],
    ) -> Result<(), FetchError> {
        for dependency in dependencies {
            log::info!("Resolving {:?}", dependency.coordinate);

            dep_map
                .entry(dependency.coordinate.clone())
                .and_modify(|vec| vec.push(dependency.revision.clone()))
                .or_insert_with(|| vec![dependency.revision.clone()]);

            let repo = cache.clone_or_update(&dependency.coordinate)?;
            let descriptor = repo.extract_descriptor(&dependency.name, &dependency.revision)?;

            repo_map
                .entry(dependency.coordinate.clone())
                .or_insert((dependency.name.clone(), repo));

            go(cache, dep_map, repo_map, &descriptor.dependencies)?;
        }

        Ok(())
    }

    let mut dep_map: HashMap<Coordinate, Vec<Revision>> = HashMap::new();
    let mut repo_map: HashMap<Coordinate, (String, ProtoRepository)> = HashMap::new();

    go(cache, &mut dep_map, &mut repo_map, dependencies)?;

    let no_conflicts = resolve_conflicts(dep_map);
    let with_repos: HashMap<Coordinate, (String, ProtoRepository, Revision)> = no_conflicts
        .into_iter()
        .filter_map(|(coordinate, revision)| {
            repo_map
                .remove(&coordinate)
                .map(|(dep_name, repo)| (coordinate, (dep_name, repo, revision)))
        })
        .collect();

    let lockfile = lock_dependencies(module_name, &with_repos)?;

    // let for_worktrees = lockfile
    //     .dependencies
    //     .clone()
    //     .into_iter()
    //     .filter_map(|locked_dep| {
    //         with_repos.remove(&locked_dep.coordinate).map(|tp| {
    //             (
    //                 locked_dep.coordinate.repository,
    //                 (tp.0, locked_dep.commit_hash),
    //             )
    //         })
    //     })
    //     .collect::<HashMap<_, _>>();

    // create_worktrees(self_module_name, out_dir, &for_worktrees)?;

    Ok(lockfile)
}

pub fn fetch<Cache: RepositoryCache>(
    cache: &Cache,
    lockfile: &LockFile,
    out_dir: &Path,
) -> Result<(), FetchError> {
    if !out_dir.exists() {
        std::fs::create_dir(out_dir)?;
    }

    if out_dir.is_dir() {
        for dep in &lockfile.dependencies {
            let repo = cache.clone_or_update(&dep.coordinate)?;
            repo.create_worktrees(&dep.name, &lockfile.module_name, &dep.commit_hash, out_dir)?;
        }

        Ok(())
    } else {
        Err(FetchError::BadOutputDir(
            out_dir.to_str().unwrap_or("").to_string(),
        ))
    }
}

fn resolve_conflicts(dep_map: HashMap<Coordinate, Vec<Revision>>) -> HashMap<Coordinate, Revision> {
    dep_map
        .into_iter()
        .filter_map(|(k, mut v)| {
            let len = v.len();

            match v.len() {
                0 => None,
                1 => Some((k, v.remove(0))),
                _ => {
                    log::warn!(
                        "discarded {} dependencies while resolving conflicts for {}",
                        len - 1,
                        k
                    );
                    Some((k, v.remove(0)))
                }
            }
        })
        .collect()
}

pub fn lock_dependencies(
    module_name: String,
    dep_map: &HashMap<Coordinate, (String, ProtoRepository, Revision)>,
) -> Result<LockFile, FetchError> {
    let mut locked_deps: Vec<LockedDependency> = Vec::new();
    for (coordinate, (name, repository, revision)) in dep_map {
        log::info!("Locking {:?} at {:?}", coordinate, revision);

        let commit_hash = repository.resolve_revision(revision)?;
        let locked_dep = LockedDependency {
            name: name.clone(),
            coordinate: coordinate.clone(),
            commit_hash,
        };

        locked_deps.push(locked_dep);
    }

    Ok(LockFile {
        module_name,
        dependencies: locked_deps,
    })
}
