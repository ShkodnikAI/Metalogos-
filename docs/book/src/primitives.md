# Eight Semantic Primitives

## Eight Semantic Primitives

| Primitive | Purpose | Analogue in other languages |
|---|---|---|
| **Entity** | Typed data with identity, confidence, relations | Structs, objects, variables |
| **Pattern** | Transformations — pure, learnable (LLM), or hybrid | Functions, API calls |
| **Flow** | Declarative pipelines with confidence-based branching | Control flow, orchestrators |
| **Memory** | Typed semantic store with FTS5 BM25 + cosine RRF hybrid recall, decay | Databases, caches, vector stores |
| **Rule** | Probabilistic rules with priority and conflict resolution | If/else chains, business logic |
| **Learn** | Training as a language operation | ML frameworks, training scripts |
| **Adapt** | Runtime self-modification with sandbox and rollback | No direct analogue |
| **Reflex** | Local neural models — train, predict, persist, distill. LLM as teacher, local head as student (closes ADR-0112) | ML inference, distillation |

---
