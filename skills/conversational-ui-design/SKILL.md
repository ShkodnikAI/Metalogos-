---
name: conversational-ui-design
description: Designing chat-based interfaces — Telegram bots, AI chat apps, conversational web UIs. Different design discipline from forms/dashboards. Turn-taking, context maintenance, error recovery in conversation, when to use buttons vs free text, command vs natural language interfaces. Critical for Fosved-style products.
---

# Conversational UI Design — Designing Dialogues, Not Screens

Telegram bots, AI chat apps, voice interfaces — they share a property: the user doesn't navigate, they converse. Traditional UX (layouts, hierarchies, navigation) doesn't fully apply. Conversation has its own discipline.

This skill is essential for Fosved-style AI offices.

## Prerequisites

- `user-task-analysis` understood
- Project type: chat bot, AI assistant, conversational web UI
- Familiarity with target platform (Telegram, web chat, voice)

## Core principle

> A good conversation doesn't make the user think about the conversation. It feels natural, recovers from mistakes, maintains context, and ends gracefully. Bad conversational UI makes the user wonder "what can I say? did it understand? did I break it?". The design discipline is removing those wonders.

## Conversation as design space

In screen-based UI, the design space is the visible elements. In conversational UI:

- **Turn-taking:** user → system → user → system
- **Context:** what's been said, what's implied
- **Affordances:** what user can do at each turn (commands? free text? buttons?)
- **State:** is there a current "task in progress"? a "back to start"?
- **Error recovery:** what happens when user input doesn't fit?
- **Personality:** consistent voice and tone

These are the design elements.

## Platform constraints

**Telegram bots:**
- Text + media in messages
- Inline keyboards (buttons attached to message)
- Reply keyboards (replaces text input area)
- Commands (`/start`, `/help`, etc.)
- Markdown formatting limited
- No persistent UI state — each message is discrete
- 4096 char message limit

**Web chat:**
- More flexibility (rich HTML in messages)
- Suggested replies / quick buttons common
- Sometimes embedded widgets in conversation
- Streaming responses for AI

**Voice (Alexa, Siri):**
- No visual — words must do all work
- TTS personality matters
- Confirmation steps for actions
- "Are you sure?" patterns mandatory

For Fosved: primarily Telegram bot. This skill leans there.

## Welcome and onboarding

User's first interaction sets expectations.

**Bad welcome:**
```
Hello! I'm a bot.
```
User: "ok... now what?"

**Good welcome:**
```
Welcome. I'm Fosved Office. I can help with:
• /analyze — strategic situation analysis (ОСП)
• /expert — meeting preparation (Эксперт)
• /scan — technology tracking (ЛЗ)
• /visual — infographics
• ...and more.

Try /analyze followed by a topic. Or just describe what you need help with.
```

Sets:
- Identity (Fosved Office)
- Capability boundaries (what it CAN do)
- How to invoke (specific commands or natural language)
- Concrete next action

For complex multi-feature bots: don't list everything. Categories + "explore" path.

## Commands vs natural language

**Commands** (`/analyze topic`):
- Pros: explicit, no parsing ambiguity, easy to remember frequently-used
- Cons: requires learning, doesn't feel conversational
- Best for: power users, frequent actions, precise inputs

**Natural language** ("проанализируй ситуацию с..."):
- Pros: discoverable, conversational, flexible
- Cons: requires NLU, ambiguity, possibly fails
- Best for: occasional users, fuzzy requests, exploration

**Hybrid (Fosved approach):**
- Both work
- Commands documented and discoverable
- Natural language → LLM routes to command
- Power users use commands, casual users use natural language
- Both feel first-class, not "fallback"

## Buttons vs text input

**Use buttons when:**
- Small finite set of choices (yes/no, options 1-4)
- Reducing typing on mobile
- Reducing parse ambiguity
- Action confirmations
- Pagination/navigation

**Use free text when:**
- Open-ended input (description, topic, name)
- Variety of possible answers
- User typing more natural than scrolling

**Don't mix unnecessarily.** "Type your name or click one of these names" — confusing. Pick one.

**Buttons within messages (inline keyboards):**
```
What would you like to analyze?
[Strategic situation]  [Technology]
[Project decision]     [Other (type in)]
```

Quick, no typing for common cases, "Other" handles long tail.

## Context maintenance

Conversation has state. Bot needs to know:
- What user said earlier (history)
- What task is in progress
- What pre-conditions established (location, account, etc.)

Without context, every message is islanded:
```
User: What's the weather?
Bot: In what city?
User: Moscow.
Bot: 18°C, sunny.
User: How about tomorrow?
Bot: In what city?  ← lost context
```

With context:
```
User: How about tomorrow?
Bot: Moscow tomorrow: 16°C, partly cloudy.
```

For Fosved: conversation history stored in `Conversation` Prisma model, accessible to LLM for context.

## Avoiding context black holes

Sometimes users return after long absence and expect bot to remember. Sometimes not.

**Pattern:** session expiration with graceful re-anchor.
```
[After 24h since last interaction]
Bot: Welcome back. Last time we discussed X. Continue, or start fresh?
[Continue X] [Start fresh]
```

Without re-anchor: "what does the bot remember?" anxiety.

## Error recovery

Things go wrong:
- User input doesn't fit
- Bot misunderstands
- External service fails
- LLM produces nonsense

**Bad error:**
```
User: Find John's phone
Bot: I don't understand.
```

**Good error:**
```
User: Find John's phone
Bot: I don't have search across personal contacts. I can:
- Search documents (try /search docs)
- Search past analyses (/archive search John)
- Search Telegram by name yourself

Or you can describe what you're trying to do?
```

Tells user:
- It didn't understand
- What it CAN do
- Path to continue

**Always offer next step.** Dead ends frustrate.

## Progress feedback

LLM responses can be slow (3-15s). User doesn't know if bot is working or broken.

**Patterns:**
- Send "typing..." indicator
- "Working on it..." quick message
- Stream tokens as generated (LLM streaming)
- Multi-step process: announce each step ("Step 1/3: gathering data...")

For Fosved bot: streaming responses where possible. Typing indicators always.

## Confirmation patterns

For destructive or significant actions:

```
Bot: I'll delete the analysis #42 ("Belarus BYN-USD"). This can't be undone. Proceed?
[Yes, delete] [Cancel]
```

For irreversible: ALWAYS confirm.
For reversible (most actions): just do it. Confirmation everywhere is noise.

## Personality

Voice should be:
- Consistent across all responses
- Appropriate to context (formal for legal advice, casual for chat)
- Matched to brand
- Avoids quirks that age poorly

For Fosved:
- Precise (not flowery)
- Direct (not apologetic)
- Russian-language-first for owner
- Technical-friendly (uses terminology correctly)
- Doesn't pretend to be human
- Doesn't pretend to know what it doesn't

Anti-pattern: bot saying "I'm not just a bot, I'm your friend!" Off-brand, off-putting.

## Constraint communication

Bot should be transparent about:
- What it can do
- What it can't do
- When it's not sure
- When it needs more info

```
Bot: I can analyze public-domain financial trends. I can't access your private financial accounts. 
For account-specific advice, your bank/accountant.
```

Better than failing on a request it can't handle.

## Multi-turn task management

For tasks requiring multiple inputs:
- Show progress (Step 2 of 4)
- Allow cancellation
- Allow back / restart
- Show summary before final commit

```
Bot: Setting up new analysis.
Step 1/3: What's the topic?
User: Belarus currency.
Bot: ✓ Topic noted.
Step 2/3: Time horizon? [3 months] [6 months] [1 year] [Custom]
User: 6 months
Bot: ✓
Step 3/3: Confidence level? [Low - exploratory] [High - investment decision]
User: High
Bot: Summary:
- Topic: Belarus currency
- Horizon: 6 months
- Confidence: High
[Start analysis] [Cancel]
```

For 2-3 step flows: linear is fine. For longer: consider redesign.

## Async operations

When task is long-running (Expert deep briefing — 5-10 min):
```
Bot: Starting deep briefing on FusionCorp. This usually takes 5-10 minutes. 
I'll send results when ready. You can continue with other tasks.
```

Then later:
```
Bot: Deep briefing on FusionCorp ready. See: [link to briefing]
```

Don't make user wait synchronously for long ops.

## Ending conversations gracefully

Conversations have implicit "end" — user satisfied or done.

**Patterns:**
- Brief sign-off ("Anything else?")
- Suggested next action ("Want to analyze related topic?")
- Just stop (silence is fine if natural)

Don't force "rate this conversation 1-5!" intrusive ratings.

## Anti-patterns

- **Verbose responses.** Wall of text. Mobile reading hard. Keep responses scannable.
- **No suggested actions.** Dead ends.
- **Inconsistent terminology.** Calling same thing "analysis" then "report" then "review". Confusing.
- **Sycophancy.** "Great question!" every time. Insincere, condescending.
- **False humanity.** "I really feel..." Bot isn't a person. Don't pretend.
- **Hidden capabilities.** Bot has features user can never discover without manual.
- **Context loss.** User has to repeat themselves.
- **No errors handling.** Bot crashes silently when input doesn't fit.
- **Buttons for everything.** No room for unique input.
- **Free text for everything.** Tedious for simple yes/no.
- **Generic help.** "/help" returns long list of commands without organization or examples.
- **Forgotten state.** "What were we discussing?" — bot doesn't know.
- **Auto-actions.** Bot does something based on inferred intent without confirming.

## Fosved bot conventions

For consistency across Fosved AI offices:

- Russian primary language (owner preference)
- Markdown sparingly (Telegram support varies)
- Inline keyboards for finite choices, free text for open
- Confirmation only for destructive (delete archives, etc.)
- Streaming for LLM-generated responses
- Status updates for multi-step (Expert briefing, etc.)
- Hashtag conventions for archive items
- Yana as personality (one consistent voice across departments)
- Department-specific terminology consistent with profiles

## Integration

- Used for any AI office or conversational interface project
- `agent-architecture` provides the technical layer
- `user-task-analysis` defines what conversations should accomplish
- `accessibility-first` ensures conversation works with assistive tech
- `qa/ai-evals-framework` tests conversation quality
- Bot UX research feeds back to `UserResearchSession`
