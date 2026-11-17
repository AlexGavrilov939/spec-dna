//! The fixture projects of `spec-dna import` (design 0.7.0, D21 to D27).
//!
//! Each project is a git repository with 1 commit. It has a `spec-dna.toml`
//! and the files of a project of Spec Kit 1.4.1 or OpenSpec 2.0.0. The text
//! is in the words of spec-dna. Only the paths, the headings and the line
//! forms follow the public layouts of the 2 tools, and no text of the 2 tools
//! is copied. A test changes a project in code (a changed sentence, an
//! unknown layout), so the files are constants and no committed folder.

use super::{Repo, VERSION};

/// The layers of the project, as `[layers.<name>]` tables.
const LAYERS: &str = "
[layers.rules]
types = [\"rule\"]

[layers.requirements]
types = [\"requirements\"]

[layers.decisions]
types = [\"decision\"]

[layers.intent]
types = [\"intent\", \"spec\", \"plan\"]
moves-to = [\"rules\", \"requirements\", \"decisions\"]
keep = \"until-finished\"
context = \"on-request\"
";

/// The layer `research`, as `--keep research` needs it.
pub const RESEARCH_LAYER: &str = "
[layers.research]
types = [\"research\"]
context = \"on-request\"
";

/// The text of the root, and the key `legacy-keys` of the first run.
pub const ROOT: &str = "
[[roots]]
path = \"docs\"
type = \"mixed\"
";

pub const ROOT_WITH_KEYS: &str = "
[[roots]]
path = \"docs\"
type = \"mixed\"
legacy-keys = [\"FR-{n}\"]
";

/// A `spec-dna.toml`: the pin, the layers, `research` if `research` is true,
/// and the root. `keys` adds the legacy key `FR-{n}`, so the first run of
/// `import` has no change to make.
pub fn config(research: bool, keys: bool) -> String {
    let mut t = format!(
        "spec-dna = \"{VERSION}\"\nprofile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{LAYERS}"
    );
    if research {
        t.push_str(RESEARCH_LAYER);
    }
    t.push_str(if keys { ROOT_WITH_KEYS } else { ROOT });
    t
}

/// A document of the project before the import. It links to a file that the
/// import moves.
pub const CORE_DOC: &str = "---
status: active
---
# Core

## Requirements

- **start-offline**: The tool starts with no network.

The first feature has [its own spec](../../specs/001-photo-albums/spec.md).
";

/// The same document, for OpenSpec.
pub const CORE_DOC_OPENSPEC: &str = "---
status: active
---
# Core

## Requirements

- **start-offline**: The tool starts with no network.

The sign-in rules are in [the auth spec](../../openspec/specs/auth/spec.md).
";

// --- Spec Kit ---

pub const SK_CONSTITUTION: &str = "# Albums Constitution

## Core Principles

### I. Library-First

Every feature starts as a small library. A library MUST work on its own and MUST have one purpose.

### II. Test-First

Tests come before the code. A test MUST fail before the code that satisfies it exists.

The red-green-refactor cycle is strict.

- Write the test.
- See it fail.
- Write the code.

## Governance

This constitution is above other practice. An amendment needs a record.

An amendment starts as a proposal in a pull request.
The proposal names the principle that it changes.
The proposal gives the reason for the change.
The owners of the project read the proposal.
Each owner can ask for a change of the text.
The amendment is accepted when all owners approve it.
The accepted text replaces the old text of the principle.
The old text stays in the history of the repository.
A change of a principle needs a note in the release notes.
Each release names the version of this constitution.

**Version**: 1.0.0 | **Ratified**: 2026-01-05
";

/// A constitution that is still the template.
pub const SK_CONSTITUTION_TEMPLATE: &str = "# [PROJECT_NAME] Constitution

## Core Principles

### [PRINCIPLE_1_NAME]

[PRINCIPLE_1_DESCRIPTION]

## Governance

[GOVERNANCE_RULES]
";

pub const SK_SPEC_1: &str = "# Feature Specification: Photo albums

**Feature Branch**: `001-photo-albums`
**Created**: 2026-01-10
**Status**: Draft
**Input**: User description: \"People group photos into albums.\"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Create an album (Priority: P1)

A person creates an album and adds photos to it.

**Acceptance Scenarios**:

1. **Given** a signed-in person, **When** they create an album, **Then** the album is listed.

### Edge Cases

- What happens when an album has no photos?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST let a person create an album with a title.
- **FR-002**: The system MUST let a person add a photo to an album. The [plan](./plan.md) says how.
- **FR-003**: The system SHOULD keep the order of the photos in an album.

### Key Entities

- **Album**: A named group of photos.
- **Photo**: One image file with its metadata.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person creates an album in less than 2 minutes.
- **SC-002**: Nine of ten people add a photo on the first try.
";

pub const SK_TASKS_1: &str = "# Tasks: Photo albums

## Phase 1: Setup

- [x] T001 Create the album module in src/albums/__init__.py
- [x] T002 [P] Add the album table in db/schema.sql

## Phase 2: User Story 1

- [x] T003 [P] [US1] Write the create endpoint in src/albums/create.py (new file)
";

pub const SK_PLAN_1: &str = "# Implementation Plan: Photo albums

The albums live in one module. See the [spec](./spec.md).
";

pub const SK_RESEARCH_1: &str = "# Research: Photo albums

Notes on the storage of photos.
";

pub const SK_CONTRACT_1: &str =
    "openapi: 3.1.0\ninfo:\n  title: Albums\n  version: 1.0.0\npaths: {}\n";

pub const SK_SPEC_2: &str = "# Feature Specification: Album sharing

**Feature Branch**: `002-album-sharing`
**Created**: 2026-01-20
**Status**: Draft

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Share an album (Priority: P1)

A person shares an album with a link.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST create a share link for an album.
- **FR-002**: The system MAY expire a share link.
";

pub const SK_TASKS_2: &str = "# Tasks: Album sharing

## Phase 1: Links

- [X] T001 Create the share link table in db/share.sql
- [x] T002 Write the link service in src/albums/share.py
";

pub const SK_SPEC_3: &str = "# Feature Specification: Album export

**Feature Branch**: `003-album-export`
**Created**: 2026-02-02
**Status**: Draft

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Export an album (Priority: P1)

A person downloads an album as one archive.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST export an album as one archive.
- **FR-002**: The system MUST offer [NEEDS CLARIFICATION: which file formats?] as formats.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person exports an album of 100 photos in less than 1 minute.
";

pub const SK_TASKS_3: &str = "# Tasks: Album export

## Phase 1: Setup

- [x] T001 Create the export module in src/export/__init__.py

## Phase 2: User Story 1

- [ ] T002 [P] [US1] Write the archive builder in src/export/zip.py
- [ ] T003 Add a progress line to the export screen
";

pub const SK_PLAN_3: &str = "# Implementation Plan: Album export

The export streams the photos into one archive.
";

pub const SK_DATA_3: &str = "# Data model: Album export

An archive has a name and a list of photos.
";

/// The files of a Spec Kit project: path and text. The tool files and the
/// 3 features: 2 done and 1 open.
pub fn speckit_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (".specify/memory/constitution.md", SK_CONSTITUTION),
        (".specify/init-options.json", "{\"ai\": \"claude\"}\n"),
        (".specify/templates/spec-template.md", "# Template\n"),
        (".specify/scripts/check.sh", "#!/bin/sh\nexit 0\n"),
        ("specs/001-photo-albums/spec.md", SK_SPEC_1),
        ("specs/001-photo-albums/tasks.md", SK_TASKS_1),
        ("specs/001-photo-albums/plan.md", SK_PLAN_1),
        ("specs/001-photo-albums/research.md", SK_RESEARCH_1),
        (
            "specs/001-photo-albums/contracts/albums.yaml",
            SK_CONTRACT_1,
        ),
        ("specs/002-album-sharing/spec.md", SK_SPEC_2),
        ("specs/002-album-sharing/tasks.md", SK_TASKS_2),
        ("specs/003-album-export/spec.md", SK_SPEC_3),
        ("specs/003-album-export/tasks.md", SK_TASKS_3),
        ("specs/003-album-export/plan.md", SK_PLAN_3),
        ("specs/003-album-export/data-model.md", SK_DATA_3),
        ("docs/requirements/core.md", CORE_DOC),
    ]
}

/// A project of Spec Kit in a repository with 1 commit. `research` and
/// `keys` are the options of [`config`].
pub fn speckit_with(research: bool, keys: bool) -> Repo {
    let repo = Repo::new();
    repo.write("spec-dna.toml", &config(research, keys));
    for (path, text) in speckit_files() {
        repo.write(path, text);
    }
    repo.commit("project");
    repo
}

/// A project of Spec Kit that needs no change of `spec-dna.toml`.
pub fn speckit() -> Repo {
    speckit_with(false, true)
}

// --- OpenSpec ---

pub const OS_SPEC_AUTH: &str = "# Auth Specification

## Purpose

People prove who they are before they use the service.

## Requirements

### Requirement: Password Sign In

The system SHALL accept a valid password for a known account.

#### Scenario: Valid password

- **WHEN** a person sends a valid password
- **THEN** the system opens a session

#### Scenario: Wrong password

- **WHEN** a person sends a wrong password
- **THEN** the system refuses the sign in

### Requirement: Session Expiry

The system SHALL end a session after 30 minutes of no use.

Long sessions are a risk.

#### Scenario: Idle session

- **WHEN** a session has no request for 30 minutes
- **THEN** the system ends the session
";

pub const OS_SPEC_BILLING: &str = "# Billing Specification

## Purpose

People pay for plans.

## Requirements

### Requirement: Invoice Issue

The system SHALL issue an invoice at the end of each month.

#### Scenario: Month end

- **WHEN** a month ends
- **THEN** the system issues one invoice for each account
";

pub const OS_CONFIG: &str = "schema: spec-driven\n";
pub const OS_PROJECT: &str = "# Project\n\nA legacy file with the notes of the project.\n";

pub const OS_PROPOSAL_2FA: &str = "## Why

Passwords alone are weak.

## What Changes

- Add a second factor at sign in.
- Change the password sign in to ask for the code.
- Remove the legacy session rule.

## Capabilities

### Modified Capabilities

- auth

## Impact

Mobile clients need an update. See the [design](./design.md).
";

pub const OS_DESIGN_2FA: &str = "## Context

The code is a 6 digit number.

## Decisions

- Use a time based code.
";

pub const OS_TASKS_2FA: &str = "## 1. Backend

- [ ] 1.1 Add the code table
- [x] 1.2 Add the code check

## 2. Clients

- [ ] 2.1 Ask for the code in the app
- [ ] Update the help page
";

pub const OS_DELTA_2FA: &str = "## ADDED Requirements

### Requirement: Second Factor

The system SHALL ask for a code after a valid password.

The code MUST come from a device of the person.

#### Scenario: Valid code

- **WHEN** a person sends the right code
- **THEN** the system opens a session

## MODIFIED Requirements

### Requirement: Password Sign In

The system SHALL accept a valid password for a known account and then ask for a code.

#### Scenario: Valid password

- **WHEN** a person sends a valid password
- **THEN** the system asks for a code

## REMOVED Requirements

### Requirement: Session Expiry

**Reason**: The new flow ends a session by the code.

**Migration**: Use the code expiry.
";

pub const OS_PROPOSAL_REPORTS: &str = "## Why

Managers want a report.

## What Changes

- Add a monthly report.
";

pub const OS_TASKS_REPORTS: &str = "## 1. Report

- [ ] 1.1 Write the report query
";

pub const OS_DELTA_REPORTS: &str = "## ADDED Requirements

### Requirement: Monthly Report

The system SHALL give a report of the invoices of one month.

#### Scenario: Report of a month

- **WHEN** a manager asks for a month
- **THEN** the system gives the report
";

pub const OS_PROPOSAL_RENAME: &str = "## Why

The name of a plan is unclear.

## What Changes

- Rename a requirement of billing.
";

pub const OS_TASKS_RENAME: &str = "## 1. Rename

- [ ] 1.1 Rename the requirement
";

pub const OS_DELTA_RENAME: &str = "## RENAMED Requirements

- FROM: `### Requirement: Invoice Issue`
- TO: `### Requirement: Invoice Creation`
";

pub const OS_PROPOSAL_ORPHAN: &str = "## Why

A rule is missing.

## What Changes

- Change a rule of a domain that has no spec.
";

pub const OS_DELTA_ORPHAN: &str = "## MODIFIED Requirements

### Requirement: Rate Limit

The system SHALL limit the sign in tries.

#### Scenario: Too many tries

- **WHEN** a person sends 10 wrong passwords
- **THEN** the system locks the account
";

pub const OS_ARCHIVED_PROPOSAL: &str = "## Why

The first sign in.

## What Changes

- Add the password sign in.
";

pub const OS_ARCHIVED_TASKS: &str = "## 1. Sign in

- [x] 1.1 Write the sign in
";

/// The files of an OpenSpec project: path and text. 2 domain specs, 4 open
/// changes (one adds a domain, one renames, one has no base requirement) and
/// an archived change.
pub fn openspec_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("openspec/config.yaml", OS_CONFIG),
        ("openspec/project.md", OS_PROJECT),
        ("openspec/specs/auth/spec.md", OS_SPEC_AUTH),
        ("openspec/specs/billing/spec.md", OS_SPEC_BILLING),
        (
            "openspec/changes/add-two-factor/proposal.md",
            OS_PROPOSAL_2FA,
        ),
        ("openspec/changes/add-two-factor/design.md", OS_DESIGN_2FA),
        ("openspec/changes/add-two-factor/tasks.md", OS_TASKS_2FA),
        (
            "openspec/changes/add-two-factor/specs/auth/spec.md",
            OS_DELTA_2FA,
        ),
        (
            "openspec/changes/add-reports/proposal.md",
            OS_PROPOSAL_REPORTS,
        ),
        ("openspec/changes/add-reports/tasks.md", OS_TASKS_REPORTS),
        (
            "openspec/changes/add-reports/specs/reports/spec.md",
            OS_DELTA_REPORTS,
        ),
        (
            "openspec/changes/rename-plan/proposal.md",
            OS_PROPOSAL_RENAME,
        ),
        ("openspec/changes/rename-plan/tasks.md", OS_TASKS_RENAME),
        (
            "openspec/changes/rename-plan/specs/billing/spec.md",
            OS_DELTA_RENAME,
        ),
        (
            "openspec/changes/limit-tries/proposal.md",
            OS_PROPOSAL_ORPHAN,
        ),
        (
            "openspec/changes/limit-tries/specs/security/spec.md",
            OS_DELTA_ORPHAN,
        ),
        (
            "openspec/changes/archive/2026-01-10-first-sign-in/proposal.md",
            OS_ARCHIVED_PROPOSAL,
        ),
        (
            "openspec/changes/archive/2026-01-10-first-sign-in/tasks.md",
            OS_ARCHIVED_TASKS,
        ),
        ("docs/requirements/core.md", CORE_DOC_OPENSPEC),
    ]
}

/// A project of OpenSpec in a repository with 1 commit.
pub fn openspec_with(research: bool, keys: bool) -> Repo {
    let repo = Repo::new();
    repo.write("spec-dna.toml", &config(research, keys));
    for (path, text) in openspec_files() {
        repo.write(path, text);
    }
    repo.commit("project");
    repo
}

/// A project of OpenSpec that needs no change of `spec-dna.toml`.
pub fn openspec() -> Repo {
    openspec_with(false, false)
}
