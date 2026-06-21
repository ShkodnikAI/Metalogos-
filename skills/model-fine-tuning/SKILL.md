---
name: model-fine-tuning
description: Practical fine-tuning of LLMs using LoRA and QLoRA — when to fine-tune vs prompt engineer, dataset preparation, training setup, evaluation, deployment. Specializes a base model on domain-specific tasks. Not pre-training (out of scope for this department).
---

# Model Fine-Tuning — Specializing LLMs for Specific Tasks

Fine-tuning teaches a model patterns from your data. Done right, a small fine-tuned model can outperform a large generic model on a narrow task. Done wrong, it wastes time and degrades the model. This skill is the practical path.

## Prerequisites

- `local-model-deployment` understood (you'll deploy the result)
- Specific narrow task where fine-tuning may help
- Dataset of input/output examples available (or possible to generate)

## Core principle

> Fine-tuning is a tool for narrow, repetitive tasks where you have data. It's NOT a shortcut to general improvement. If your task is broad reasoning, prompt engineer instead. If your task is "produce output in specific format on specific domain", fine-tune.

## When to fine-tune vs alternatives

**Fine-tune when:**
- Task is narrow and well-defined
- You have 100+ high-quality input/output examples
- Prompt engineering hit a ceiling
- Output format is critical and consistent
- Domain has specific terminology/style

**Prompt engineer instead when:**
- Task is broad or reasoning-heavy
- You have <100 examples
- Underlying capability not in base model (fine-tuning can't add knowledge it lacks)
- You're early — get prompt working first, fine-tune later if needed

**Use RAG instead when:**
- Need to inject specific factual knowledge
- Knowledge updates frequently
- Source citations required
- Variety of knowledge access patterns

**Use external service instead when:**
- One-off task — fine-tuning overhead not worth it
- Need state-of-the-art quality — frontier model better than any fine-tune

## Methods

**Full fine-tune:** all model weights updated. Requires huge GPU. Rare for practical use.

**LoRA (Low-Rank Adaptation):** add small "adapter" layers, train only those. ~1% of parameters trainable. Fast, cheap, effective. Default for fine-tuning.

**QLoRA:** LoRA but base model in 4-bit. Fits larger base models on smaller GPUs.

**Prompt tuning:** train "soft prompts" prepended to input. Smaller still but less powerful than LoRA.

For Fosved: **QLoRA on Llama 3 7B-70B** as practical default.

## Hardware

For QLoRA training:

| Base model | VRAM for QLoRA |
|-----------|----------------|
| Llama 3 7B | 8-12 GB (consumer GPU works) |
| Llama 3 13B | 16-24 GB |
| Llama 3 70B | 48 GB (2x consumer or 1x pro GPU) |

Training time:
- 7B model, 1000 examples: 1-2 hours on RTX 4090
- 70B model, 1000 examples: 8-15 hours on dual RTX 4090

Cloud GPU rental (RunPod, Vast.ai) viable: ~$1-2/hour, total cost $5-30 per training run.

## Dataset preparation

**Format:** JSONL with input/output pairs:

```jsonl
{"messages": [{"role": "user", "content": "input 1"}, {"role": "assistant", "content": "output 1"}]}
{"messages": [{"role": "user", "content": "input 2"}, {"role": "assistant", "content": "output 2"}]}
```

**Dataset size guidelines:**
- 50-100 examples: bare minimum, results unstable
- 200-500: usable for narrow tasks
- 1000-5000: good for most tasks
- 10000+: serious training

**Quality > quantity.** 200 high-quality > 2000 mediocre.

**Dataset hygiene:**
- Remove duplicates
- Diverse inputs (don't all start with same phrase)
- Diverse outputs (don't all use same template)
- Verify each output (manual spot-check 10-20%)
- Hold out 10-20% as test set (never used in training)

**Synthetic dataset generation:**
Use a stronger model (Claude Opus) to generate training data for a weaker model (Llama 3 7B):

```typescript
// Generate dataset
for (const input of inputSeeds) {
  const ideal = await claude.call([
    { role: 'user', content: `Generate ideal response for: ${input}` }
  ]);
  dataset.push({ input, output: ideal.text });
}
```

Then fine-tune Llama 3 on dataset. Now small local model approximates Opus quality on this task at fraction of cost.

## Training setup

Using HuggingFace transformers + peft library:

```python
# train_lora.py
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import LoraConfig, get_peft_model, TaskType
from datasets import load_dataset
from trl import SFTTrainer

# Base model (4-bit for QLoRA)
model = AutoModelForCausalLM.from_pretrained(
    "meta-llama/Llama-3.3-8B",
    load_in_4bit=True,
    device_map="auto"
)
tokenizer = AutoTokenizer.from_pretrained("meta-llama/Llama-3.3-8B")

# LoRA config
lora_config = LoraConfig(
    task_type=TaskType.CAUSAL_LM,
    r=16,                  # rank: higher = more capacity, slower
    lora_alpha=32,         # scaling: typically 2x rank
    lora_dropout=0.1,
    target_modules=["q_proj", "v_proj"]  # which layers to adapt
)
model = get_peft_model(model, lora_config)

# Dataset
dataset = load_dataset("json", data_files="data/train.jsonl")

# Train
trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=dataset["train"],
    max_seq_length=2048,
    num_train_epochs=3,
    per_device_train_batch_size=4,
    gradient_accumulation_steps=4,
    learning_rate=2e-4,
    output_dir="./lora-output"
)
trainer.train()
trainer.save_model("./lora-final")
```

Run on rented GPU or local workstation.

## Hyperparameters

Common defaults that work:
- **LoRA rank (r):** 8-32. Higher = more capacity but more memory.
- **Alpha:** 2x rank.
- **Learning rate:** 1e-4 to 5e-4.
- **Epochs:** 2-5. More = overfit risk.
- **Batch size:** match GPU memory. Use gradient accumulation if needed.

If loss not decreasing: increase learning rate or rank.
If loss decreasing but eval bad: overfitting — fewer epochs or more data.

## Evaluation

**Loss curve:** training loss decreases (expected). Validation loss decreases then plateaus or rises (early stopping point).

**Held-out test:** run model on 10-20% held-out data, compare to baseline (un-fine-tuned).

**Real eval:** beyond loss, measure what you care about:
- For classification: accuracy, F1
- For generation: human eval, reference comparison, AI-as-judge
- For specific format: format compliance rate

Use Promptfoo or custom eval (see `qa/ai-evals-framework`).

**Regression check:** ensure fine-tune didn't break general capability. Run on general benchmark — should not degrade significantly.

## Deployment

LoRA adapter is small (~50-500 MB). Two deployment patterns:

**Pattern 1: Merged.** Apply LoRA to base, save full model. Deploy via Ollama:
```bash
# Convert to GGUF
python convert.py --lora-path lora-final --base-model llama-3-8b
# Then ollama create custom-model -f Modelfile
```

Pros: single model, simple ops.
Cons: locked to fine-tune.

**Pattern 2: Dynamic.** Load base + apply LoRA at runtime. More flexibility (swap adapters per request).

Pros: A/B test adapters, deploy multiple specializations.
Cons: more complex serving.

For Fosved: start with merged. Move to dynamic if multiple fine-tunes needed.

## Cost analysis

**One-time training:**
- Data prep: 5-20 hours human time (label/clean dataset)
- Training: 1-15 hours GPU
- Eval: 2-5 hours
- Total: ~$10-50 cloud GPU + significant time

**Ongoing:**
- Inference: same cost as base model
- Maintenance: re-train when data shifts

**Break-even vs cloud:**
At what volume does fine-tuned 7B match Opus on the task?
If fine-tune brings 7B to Opus quality on narrow task:
- 7B Q4 on local GPU: ~$0.0001 per request
- Opus cloud: ~$0.001-0.01 per request

10-100x cost savings at scale. But only after fine-tune investment.

## Pitfalls

**Catastrophic forgetting:** fine-tuning on narrow domain can degrade general capability. Mitigation: include some general data in training mix.

**Format learning, not task learning:** model learns to mimic output format but not actually solve task. Mitigation: diverse examples, eval on novel inputs.

**Overfitting:** model memorizes training data, fails on similar but unseen. Mitigation: held-out test set, regularization, fewer epochs.

**Distribution shift:** fine-tuned on one type of data, deployed on different distribution. Mitigation: ensure training data represents production distribution.

**Lost-in-the-middle:** for long contexts, fine-tuned models can still struggle with middle of context. Not fixed by fine-tuning.

## Realistic Fosved fine-tuning candidates

Where it could help:
- **OSP V2 5-level topology formatting:** consistent structure across analyses
- **ADR generation:** Architecture Decision Records in standard format
- **Conventional commit messages:** generate from diffs
- **Combat questions formatting:** Expert briefing structure
- **Test case generation:** from feature description

Where it won't help:
- **Strategic reasoning** (OSP analysis content): needs frontier model
- **Domain knowledge** (technology details): use RAG instead
- **Novel scenarios:** fine-tuning specializes, doesn't generalize

## Anti-patterns

- **Fine-tune everything.** Premature. Try prompt engineering first.
- **Tiny dataset.** 30 examples and expecting magic. Need hundreds minimum.
- **No eval.** Train, deploy, hope. Without evaluation, you don't know if it helps or hurts.
- **Same data train and eval.** Inflated metrics, real-world fails.
- **Stale fine-tune.** World changes, model frozen. Schedule re-training when data distribution shifts.
- **Hidden version.** Fine-tuned model with no versioning, no documentation, mystery in production.
- **Too many epochs.** Overfit, training loss great, eval terrible.
- **Wrong base model.** Fine-tune Llama 3 7B for code, when CodeLlama starts higher.
- **No regression check.** Fine-tune broke general capability silently.

## Integration

- `local-model-deployment` deploys the result
- `llm-integration` provider abstraction works for fine-tuned models
- `qa/ai-evals-framework` runs evals to validate
- `tech-radar-maintenance` tracks LoRA/QLoRA tool versions
- ADRs document fine-tune decisions
