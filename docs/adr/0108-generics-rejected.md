# ADR-0108: Generics — не вводить, подтверждает решение ADR-0011

> **Status:** Rejected (reaffirmed)  
> **Date:** 2026-08-21  
> **Decision by:** owner  
> **Precedent:** ADR-0011 (type inference — original decision),
> ADR-0105 (VM experimental scope — same principle)

## Context

Внешний аудит предложил ввести generics как приоритетный шаг к
«полноценной» системе типов, не зная о существующем решении.

`ADR-0011` (Status: Implemented) уже решило этот вопрос для текущей
фазы, с явным сравнением prior art (Hindley-Milner vs constraint-based
vs explicit annotations) и обоснованием:

> «Metalogos has explicit type annotations on patterns and entities,
> making forward-propagation through the flow pipeline sufficient for
> Phase 2.»

Это не пробел — принятое решение со статусом «Implemented».

## Decision

**Подтверждается, не пересматривается.** Generics не вводятся.

Единственное найденное основание для будущего пересмотра — конкретный,
воспроизводимый случай, где явная типизация реально мешает (пример-
кандидат: `std/collections.mlog` жёстко типизирован под `String`,
`first(items: List) -> String` — если понадобится тот же паттерн для
чисел или структур, это будет реальный триггер). Абстрактное
«современным языкам нужны generics» — не таким основанием.

## Consequences

- Грамматика и AST остаются без параметров типа.
- Явные типы на паттернах/сущностях + forward-propagation остаются
  единственным механизмом проверки типов.
- Пересмотр — только при демонстрации конкретного случая, не из
  общих соображений полноты (тот же принцип, что ADR-0105 применил
  к VM).

## Related

- ADR-0011 — исходное решение и полное обоснование (не переписывается,
  этот ADR только подтверждает его актуальность после внешнего аудита)
- ADR-0105 — тот же принцип «не чинить без доказанного спроса»
