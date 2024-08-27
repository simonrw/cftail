# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.2](https://github.com/simonrw/cftail/compare/v0.9.1...v0.9.2) - 2024-08-27

### Added
- deduplicate events based on id ([#197](https://github.com/simonrw/cftail/pull/197))

### Other
- custom release token ([#198](https://github.com/simonrw/cftail/pull/198))
- *(deps)* bump serde from 1.0.208 to 1.0.209 ([#196](https://github.com/simonrw/cftail/pull/196))
- *(deps)* bump aws-sdk-cloudformation from 1.43.0 to 1.44.0 ([#195](https://github.com/simonrw/cftail/pull/195))
- *(deps)* bump aws-sdk-cloudformation from 1.42.0 to 1.43.0 ([#193](https://github.com/simonrw/cftail/pull/193))
- *(deps)* bump serde from 1.0.207 to 1.0.208 ([#194](https://github.com/simonrw/cftail/pull/194))
- *(deps)* bump aws-smithy-types from 1.2.1 to 1.2.2 in the aws-dependencies group ([#192](https://github.com/simonrw/cftail/pull/192))
- fix automerge CI workflow ([#191](https://github.com/simonrw/cftail/pull/191))
- *(deps)* bump aws-smithy-types from 1.2.0 to 1.2.1 in the aws-dependencies group ([#190](https://github.com/simonrw/cftail/pull/190))
- *(deps)* bump aws-config from 1.5.4 to 1.5.5 in the aws-dependencies group ([#186](https://github.com/simonrw/cftail/pull/186))
- *(deps)* bump aws-sdk-cloudformation from 1.41.0 to 1.42.0 ([#187](https://github.com/simonrw/cftail/pull/187))
- *(deps)* bump serde from 1.0.205 to 1.0.207 ([#189](https://github.com/simonrw/cftail/pull/189))
- *(deps)* bump serde from 1.0.204 to 1.0.205 ([#185](https://github.com/simonrw/cftail/pull/185))
- *(deps)* bump env_logger from 0.11.4 to 0.11.5 ([#184](https://github.com/simonrw/cftail/pull/184))
- *(deps)* bump env_logger from 0.11.3 to 0.11.4 ([#183](https://github.com/simonrw/cftail/pull/183))
- *(deps)* bump aws-sdk-cloudformation from 1.40.0 to 1.41.0 ([#182](https://github.com/simonrw/cftail/pull/182))
- configure release-plz ([#181](https://github.com/simonrw/cftail/pull/181))
- fix some warnings ([#179](https://github.com/simonrw/cftail/pull/179))

### Added

- Support exiting the poll loop early with `--exit-when-stack-deploys` flag [#81]

## [0.9.0]

### Added

- Support for `--endpoint-url` for use with e.g. localstack


[Unreleased]: https://github.com/simonrw/rust-fitsio/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/simonrw/rust-fitsio/compare/v0.8.0...v0.9.0
[#81]: https://github.com/simonrw/cftail/pull/81
