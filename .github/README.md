# How the release process is working in this repository

1. Develop features and bugfixes from persistent `develop` branch, opening branches appropriately and merging them back to `develop`. Direct push to `develop` and `master` branches must be prohibited completely by settings of repository (Settings-Branches or Settings-Rules-Rulesets).
2. When changes are ready to be released (because you decided so or according to some roadmap), create `release` branch from `develop` and push it to repository:

```shell
$ git checkout develop      # Probably done already
$ git pull                  # Get updates from others, maybe
$ git checkout -b release   # Create new branch `release`
$ git push origin release   # Push it to github server
```

3. CI/CD process on GitHub Server (GitHub Workflow `PCurl-Get build`) is triggered by push event on `release` branch and will checkout repository, prepare build tools for different platforms, bump version with git-cliff and conventional commits (build will have correct next version defi), run tests, build software, store built artifacts, create CHANGELOG, new version tag and create Pull Request (PR) to `master` branch.
4. If everything looks OK and you decided to actually make release, just approve PR in GitHub UI or CLI. This will merge `release` branch onto `master` branch and trigger next Action which will create Release with release notes and merge changes back to `develop` branch, deleting `release` branch (see list item 8 below).
5. During release process all activity on `develop` branch must be ceased (branch frozen), so there will be no conflicts on back merge from `master` to `develop`. To achieve this just don't approve any PRs to `develop` branch until Release is over.
6. If, for some reason, you decided to cancel Release at this stage, just cancel PR to `master` branch and delete created `release` branch.
7. If you want to add some last-minute hot-fixes to Release you do not need to cancel PR to `master`. Just add needed changes and commit them on `release` branch -- PR will be updated accordingly and version tag moved to last commit on the `release` branch.
8. Actual Release is created by workflow `PCurl-Get Release` on the `master` branch which is triggered on any push to `master` but will be self-canceled if is fails to find new version tag (in format 'v*.*.*') on last commit or commit before last (HEAD or HEAD^1). Version tag must be equal to version bumped with git-cliff on previous build workflow. Starting conditions of Release workflow also include version tag push event (see list item 10 below).
9. The reason to check previous commit on the `master` branch for version tag is, that: when Release PR is merged into the `master` branch, it creates next commit by itself and version tag (created by `PCurl-Get build` workflow) stays on previous commit: last commit of the `release` branch.
10. Checking two last commits and triggering workflow on push and tag events allows creation of new Release by pushing annotated version tag to the `master` branch manually. It may help in case of `PCurl-Get Release` workflow failed for some reason (after fixing the cause of failure in workflow). See also Important Notes 1.
11. There is a condition in `PCurl-Get Release` workflow to prevent attempt to create of already created release.

## IMPORTANT NOTES

1. Build assets are stored in temporary GitHub storage (by default for 90 days, maximum), so DO NOT let Release PR for more than 90 days without approval. There is no (yet) process of rebuilding release assets in case of their expiration, so `PCurl-Get Release` workflow will fail and you will have to restart Release from the beginning.
2. Do not update workflow files in `.github/` directory on the `develop` branch. Instead, work on workflows on the separate `workflow` branch commit changes with `chore(github): ...` comments (so they will not appear in CHANGELOG, release notes have all commits in release) and merge them into `master`. This is to prevent merge conflicts during automated workflows and keep development of the software separate from workflows development.
<!--TODO: 1. To ensure this policy there is a filter in `PCurl-Get build` workflow that will ignore content of `.github/workflows` during release PR preparation.-->
3. You can check parameters of last created release as environment variables in `.github/config` file. This file will be recreated each time `PCurl-Get build` workflow runs. Value of new version tag is based solely on existing version tags in repository and decided by `git-cliff` during Release preparation.

## Workflow CHANGELOG

There is a separate changelog for workflows in this repository (like we are having monorepo including main software and workflows). Changelogs for main software in workflows are generated during release process by `git-cliff` with `--exclude-path=".github/**/*"` argument. Changelogs and releases for workflows are generated manually with this commands in repository:

```shell
$ git checkout master
$ git pull
$ git checkout workflow || git checkout -b workflow
$ cd .github
#
# Make changes to workflow files
#
# Assume only files in .github/ directory changed on this branch
$ git add .github/workflows
$ git commit -m "chore(github): workflow updates"
$ export NEW_WF_VERSION_TAG=$(git-cliff --tag-pattern "^workflow-v\\d+\\.\\d+\\.\\d+$" --bumped-version)
$ export NEW_WF_VERSION=$(echo ${NEW_WF_VERSION_TAG} | sed -E "s/^workflow-v//")
$ git-cliff --bump --tag-pattern "^workflow-v\\d+\\.\\d+\\.\\d+$" --output CHANGELOG.md --exclude-path=".github/config"
# Assume only files in .github/ directory changed on this branch
$ git commit -m "chore(version): CHANGELOG for ${NEW_WF_VERSION_TAG}" -- CHANGELOG.md
$ git tag -f -a "${NEW_WF_VERSION_TAG}" -m "Release ${NEW_WF_VERSION_TAG}"
$ git push origin "${NEW_WF_VERSION_TAG}"
$ git push origin workflow
```

After that create Pull Request in GitHub UI or with CLI from `workflow` branch and merge it into `master` branch. If work on workflow files is going to continue it is a good idea to pull all changes from master back into `workflow` branch with this commands in repository:

```shell
$ git checkout master
$ git pull
$ git checkout workflow
$ git merge master
```

Usually it should run without merging conflicts if all rules in this README are observed.

Use conventional commits to commit changes to workflow files normally (with `feat` and `fix`). They will not appear on main software changelog because of `--exclude-path` argument to `git-cliff`.
<!-- -->