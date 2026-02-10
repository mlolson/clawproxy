# ClawProxy Task Tracker

## Overview

Total: 37 tasks | Claude: 27 (73%) | Human: 10 (27%)

## Task List

| ID | Task | Assignee | Dependencies | Status |
|----|------|----------|--------------|--------|
| **Phase 1: Project Setup** |||||
| 1.1 | Initialize Cargo workspace | Claude | - | Done |
| 1.2 | Set up shared library structure | Claude | 1.1 | Done |
| 1.3 | Set up logging | Claude | 1.2 | Done |
| **Phase 2: Configuration** |||||
| 2.1 | Implement config loading | Claude | 1.3 | Done |
| 2.2 | Implement secrets loading | Claude | 2.1 | Done |
| 2.3 | Implement config validation | Human | 2.1, 2.2 | Done |
| **Phase 3: Sandbox Implementation** |||||
| 3.1 | Define sandbox trait and types | Claude | 1.3 | Done |
| 3.2 | Implement macOS sandbox | Human | 3.1 | Done |
| 3.3 | Implement Linux sandbox | Claude | 3.1 | Done |
| 3.4 | Implement launcher binary | Claude | 3.1, 2.1 | Done |
| 3.5 | Test sandbox effectiveness on mac | Human | 3.2, 3.3, 3.4 | Done |
| 3.6 | Test sandbox effectiveness on linux | Human | 3.2, 3.3, 3.4 | Open |
| **Phase 4: Proxy Implementation** |||||
| 4.1 | Implement router | Claude | 2.1 | Done |
| 4.2 | Implement token substitution | Claude | 2.2 | Done |
| 4.3 | Implement HTTP server | Claude | 4.1 | Done |
| 4.4 | Implement request forwarding | Claude | 4.3 | Done |
| 4.5 | Implement response streaming | Claude | 4.4 | Done |
| 4.6 | Implement error handling | Human | 4.3, 4.4, 4.5 | Done |
| **Phase 5: CLI Implementation** |||||
| 5.1 | Implement `clawproxy init` | Claude | 2.1 | Open |
| 5.2 | Implement `clawproxy secret set` | Claude | 2.2, 5.1 | Open |
| 5.3 | Implement `clawproxy secret list` | Human | 5.2 | Open |
| 5.4 | Implement `clawproxy secret delete` | Claude | 5.2 | Open |
| 5.5 | Implement `clawproxy start` | Claude | 4.5, 5.1 | Open |
| 5.6 | Implement `clawproxy status` | Human | 5.5 | Open |
| 5.7 | Implement `clawproxy configure-openclaw` | Claude | 5.1, 5.2, 3.4 | Open |
| **Phase 6: Testing** |||||
| 6.1 | Unit tests — Config | Claude | 2.3 | Open |
| 6.2 | Unit tests — Router | Claude | 4.1 | Open |
| 6.3 | Unit tests — Substitution | Human | 4.2 | Open |
| 6.4 | Integration tests — Proxy | Claude | 4.6 | Open |
| 6.5 | Integration tests — Sandbox | Human | 3.5 | Open |
| 6.6 | End-to-end tests | Claude | 6.4, 6.5 | Open |
| **Phase 7: Distribution** |||||
| 7.1 | Build release binaries | Human | 6.6 | Open |
| 7.2 | Create install script | Claude | 7.1 | Open |
| 7.3 | Create Homebrew formula | Claude | 7.1 | Open |
| 7.4 | Document installation | Claude | 7.2, 7.3 | Open |
| **Phase 8: Documentation** |||||
| 8.1 | Write README.md | Claude | 5.7 | Open |
| 8.2 | Write SECURITY.md | Human | 3.5, 6.5 | Open |
| 8.3 | Add --help documentation | Claude | 5.7 | Open |

## Dependency Graph (Critical Path)

```
1.1 → 1.2 → 1.3 → 2.1 → 2.2 → 2.3
                    ↓      ↓
                   4.1    4.2
                    ↓
              4.3 → 4.4 → 4.5 → 4.6 → 6.4
                                        ↓
1.3 → 3.1 → 3.2/3.3 → 3.4 → 3.5 → 6.5 → 6.6 → 7.1 → 7.2/7.3 → 7.4
              ↓
         5.1 → 5.2 → 5.3/5.4
          ↓     ↓
         5.5   5.7 → 8.1/8.3
          ↓
         5.6
```

## Human Tasks Summary

Human tasks are chosen to provide learning opportunities and require manual verification:

1. **2.3 Config validation** - Understand the config structure deeply
2. **3.2 macOS sandbox** - Platform-specific, requires manual testing
3. **3.5 Test sandbox** - Manual verification on both platforms
4. **4.6 Proxy error handling** - Understand error flows end-to-end
5. **5.3 Secret list** - Simple CLI task, good starter
6. **5.6 Status command** - Simple CLI task, good starter
7. **6.3 Unit tests substitution** - Learn testing patterns
8. **6.5 Integration tests sandbox** - Platform verification
9. **7.1 Build release binaries** - Manual cross-platform testing
10. **8.2 SECURITY.md** - Security review requires human judgment

## File Locations

- Open Claude tasks: `tasks/claude/`
- Open Human tasks: `tasks/human/`
- Closed tasks: `tasks/closed/`
