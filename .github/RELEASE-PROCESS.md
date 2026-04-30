# How to release software on GitHub

1. Develop features and bugfixes from persistent `develop` branch, opening branches appropriately and merging them back to `develop`. Direct push to `develop` branch must be prohibited completely or limited to very small group of developers.
2. When release is ready (because you fill so or according to some roadmap), create `release` branch from `develop` and push it to repository:

```shell
$ git checkout develop      # Probably done already
$ git pull                  # Get updates from others, maybe
$ git checkout -b release   # Create new branch `release`
$ git push origin release   # Push it to github server
```

3. CI/CD process on GitHub Server (GitHub Actions) will checkout repository, prepare build tools for different platforms, bump version with git-cliff and conventional commits (so build would contain correct next version), run tests, build software, store built artifacts, create CHANGELOG, new version tag and Pull Request (PR) to `master` branch.
4. If everything looks OK and you decided to actually make release, just approve it in GitHub interface. This will merge `release` branch onto `master` branch (deleting `release` branch) and trigger next Action there. Which will create Release with release notes and merge changes back to `develop` branch.
5. During release process all activity on `develop` branch must be ceased (branch frozen), so there will be no conflicts on back merge from `master` to `develop`. To achieve this just don't approve any PRs to `develop` branch until Release is over.
6. If, for some reason, you decided to cancel Release at this stage, just cancel PR to `master` branch and delete created `release` branch.
7. If you just want to add some last-minute hotfixes to Release but then actually finish already started release process, you can cancel PR to `master` and add some commits to `release` branch. Just remember, that on each push to `release` the process of Release will be started over again (and hence PR to `master` created).
8. CI/CD process is smart enough to move version tag existing on the `release` branch to the last branch's commit and not to create another one.
<!-- -->