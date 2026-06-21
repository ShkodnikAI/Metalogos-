---
name: forecast-verification-protocol
description: Compare past forecast against actual reality, calculate MAPE per period, identify systematic forecasting errors. Use at verification date of any forecast.
tier: 1
status: placeholder
---

# forecast-verification-protocol — PLACEHOLDER

⚠ **Этот скилл — структурный плейсхолдер.** Имеет правильную структуру (frontmatter, секции), но содержание методологии **не написано**. При загрузке возвращает базовую методологию через parent department profile + указание на сам факт плейсхолдера.

## Prerequisites

- Юрисдикция и бизнес-форма определены
- Подгружены соответствующие knowledge файлы
- (TODO: дополнить специфичными prerequisites)

## Core principle

> **TODO: одно предложение с инверсией интуиции.**

(TODO: 2-3 абзаца раскрывающие принцип)

## The procedure

### Step 1 — TODO
TODO: первый шаг

### Step 2 — TODO
TODO: второй шаг

### Step 3 — Сохранить через saveResult()
```js
await finance.saveResult({
  shortTitle: '<one-line summary>',
  fullResult: '<full result>',
  confidence: 'medium',
  verificationDate: <calculated>,
  jurisdiction,
  businessForm,
});
```

## Worked example

(TODO: реальный кейс с числами)

## Anti-patterns

- **TODO antipattern 1** — описание + countermeasure
- **TODO antipattern 2** — то же
- **TODO antipattern 3** — то же

## Output template

```
TODO: structured template
```

## When NOT to use this skill

- TODO: границы

## Integration with finance

Этот скилл — Tier 1. Загружается: TODO когда именно.

Использует: TODO какие модели БД, какие knowledge файлы.

Связь с метрикой: TODO.

---

## Статус наполнения

Этот SKILL.md создан как структурно валидный плейсхолдер в v1.0 Finance pack. Для production использования требуется:

1. Заполнить Core principle (главный принцип одной фразой + 2-3 абзаца)
2. Расписать Procedure (5-10 атомарных шагов)
3. Добавить Worked example с конкретными числами
4. Минимум 3 anti-patterns с countermeasures
5. Output template для парсера

Без этого скилл работает только как маркер «такая тема обрабатывается этим отделом», но методологию не несёт.

