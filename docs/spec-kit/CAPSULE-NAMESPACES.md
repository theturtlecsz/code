# Capsule URI Namespaces

Logical URI scheme for capsule storage (mv2:// protocol).

***

## URI Format

```
mv2://<workspace>/<spec>/<run>/<type>/<path>
```

| Component   | Description             | Example               |
| ----------- | ----------------------- | --------------------- |
| `workspace` | Workspace identifier    | `default`             |
| `spec`      | SPEC identifier         | `SPEC-KIT-042`        |
| `run`       | Intake/run identifier   | `abc123-def456`       |
| `type`      | Object type (see below) | `artifact`            |
| `path`      | Object-specific path    | `intake/answers.json` |

***

## Object Types

The capsule stores six object types:

| Type         | Description                               | SPEC         |
| ------------ | ----------------------------------------- | ------------ |
| `artifact`   | Intake artifacts, exports, grounding data | SPEC-KIT-974 |
| `event`      | Replayable audit events                   | SPEC-KIT-975 |
| `checkpoint` | Stage boundary commits                    | D18          |
| `policy`     | Policy snapshots                          | SPEC-KIT-977 |
| `card`       | Memory cards (logic mesh)                 | SPEC-KIT-976 |
| `edge`       | Logic mesh edges                          | SPEC-KIT-976 |

***

## URI Examples

### Artifacts

```
mv2://default/SPEC-KIT-042/abc123/artifact/intake/answers.json
mv2://default/SPEC-KIT-042/abc123/artifact/intake/brief.json
mv2://default/SPEC-KIT-042/abc123/artifact/intake/ace_frame.json
```

### Events

```
mv2://default/SPEC-KIT-042/abc123/event/0001
mv2://default/SPEC-KIT-042/abc123/event/0002
```

### Checkpoints

```
mv2://default/checkpoint/chk_abc123
mv2://default/checkpoint/chk_stage1_complete
```

### Policy

```
mv2://default/policy/pol_abc123
mv2://default/policy/model_policy_v1
```

### Memory (Logic Mesh)

```
mv2://default/SPEC-KIT-042/abc123/card/card_001
mv2://default/SPEC-KIT-042/abc123/edge/edge_001
```

***

## Grounding Artifact Namespaces

Deep mode captures grounding artifacts under the following structure:

```
mv2://<workspace>/<spec>/<intake_id>/artifact/intake/grounding/
├── harvest/
│   ├── churn_matrix.md       # File change frequency analysis
│   ├── complexity_map.md     # Code complexity metrics
│   └── repo_skeleton.md      # Repository structure
└── intel/
    ├── snapshot.json         # Project snapshot
    └── feeds/
        └── <feed_name>.json  # Intelligence feeds
```

### Harvest Artifacts

| Artifact            | Description                                       |
| ------------------- | ------------------------------------------------- |
| `churn_matrix.md`   | File change frequency analysis from git history   |
| `complexity_map.md` | Cyclomatic complexity and maintainability metrics |
| `repo_skeleton.md`  | Repository structure and key file identification  |

### Intel Artifacts

| Artifact        | Description                                 |
| --------------- | ------------------------------------------- |
| `snapshot.json` | Project snapshot with dependencies, configs |
| `feeds/*.json`  | Intelligence feeds (crates.io, npm, etc.)   |

***

## Invariants

From SPEC.md decision register:

| ID   | Invariant                                               |
| ---- | ------------------------------------------------------- |
| D70  | Logical URIs are immutable once returned                |
| D71  | Physical IDs never treated as stable keys               |
| D72  | All cross-object references use logical URIs            |
| D103 | All stored records contain `logical_uri` field          |
| D104 | Single entry point for URI resolution (`CapsuleHandle`) |

**Key principle**: Always reference artifacts by logical URI, never by physical storage ID.

***

## Special URI Formats

### Workspace-level objects

Objects without a specific spec/run:

```
mv2://<workspace>/checkpoint/<checkpoint_id>
mv2://<workspace>/policy/<policy_id>
```

### Export archives

Exported archives use the `.mv2` or `.mv2e` (encrypted) extension:

```
SPEC-KIT-042_20260125.mv2      # Unencrypted export
SPEC-KIT-042_20260125.mv2e     # Encrypted export (age)
```

***

## See Also

* [COMMANDS.md](COMMANDS.md) - Command reference
* [../OPERATIONS.md](../OPERATIONS.md) - Operational playbook
* [../../codex-rs/SPEC.md](../../codex-rs/SPEC.md) - Invariants and decisions
