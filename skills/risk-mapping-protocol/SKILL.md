---
name: risk-mapping-protocol
description: Систематическое выявление и картирование правовых рисков сделки. Применяется при любом запросе на анализ договора, оценку сделки, due diligence или формулировку «что может пойти не так».
tier: 1
---

# Risk Mapping Protocol — Карта правовых рисков

Это **первый скилл Legal**. Любая работа с договором начинается с построения карты рисков — без неё drafting превращается в копирование шаблона, а анализ чужого договора — в чтение текста, а не в выявление дыр.

## Prerequisites

- Юрисдикция определена (BY/RU/EU/US) — загружен `library/knowledge/legal/jurisdictions/<JX>.md`
- Бизнес-форма каждой стороны определена — загружены `business-forms/<FORM>.md`
- Тип документа известен (NDA / MSA / SLA / TZ / прочее)
- Понятна **позиция владельца** в сделке: продавец / покупатель / заказчик / исполнитель / партнёр

## Core principle

> Юрист-параноик читает не «что сказано», а «**кому это вредно при плохом сценарии**». Каждая формулировка договора — это либо защита от риска, либо создание риска. Нейтральных формулировок не бывает.

В договоре нет «безобидных» пунктов. Даже «срок договора 1 год» — это либо защита (если ты хочешь возможность не продлевать), либо риск (если ты вложил инвестиции и сторона может уйти).

Карта рисков **не симметрична**: одно и то же положение защищает одну сторону и вредит другой. Юрист обслуживает **свою сторону** (с честным указанием перекосов).

## The procedure

### Step 1 — Контекст сделки

Зафиксировать в Risk Register:
- **Стороны** (название, юрисдикция, бизнес-форма)
- **Тип сделки** (NDA, MSA, SLA, TZ, и т.д.)
- **Сумма сделки** (порядок величины, не точная)
- **Срок исполнения** (одноразово / длящаяся)
- **Критичность для бизнеса** (нет проекта без этого / могу обойтись без / низкий приоритет)
- **Позиция владельца** в сделке

### Step 2 — Идентификация **категорий риска**

Прогнать сделку через **четыре категории**:

**1. Financial risks** (финансовые)
- Неоплата / задержка платежа
- Penalty без cap
- Indemnity unlimited
- Скрытые расходы
- Валютные риски (см. также Finance multi-currency-handling)
- Налоговые последствия

**2. Operational risks** (операционные)
- Сорванные сроки (со стороны контрагента)
- Качество не соответствует
- Зависимость от ключевого человека на стороне контрагента
- Сложность смены контрагента

**3. Reputational risks** (репутационные)
- Конфиденциальность нарушена
- Публичный спор
- Связь с одиозным контрагентом
- Споры в открытых судах

**4. Regulatory / Compliance risks** (регуляторные)
- GDPR / privacy violations
- Sanctions exposure
- Tax authorities scrutiny
- Industry-specific (financial services, healthcare, и т.д.)

Для каждой категории — конкретные риски этой сделки.

### Step 3 — Оценка каждого риска

Для каждого риска — **три параметра**:

- **Likelihood** (вероятность): низкая / средняя / высокая
- **Impact** (последствия): малые / средние / критические
- **Mitigation** (как защититься): какая формулировка / процедура

**Risk score** = likelihood × impact:
- **Critical** (HxH или HxM или MxH) — обязательно адресовать в договоре
- **Major** (MxM, HxL, LxH) — рекомендуется адресовать
- **Minor** (LxM, MxL, LxL) — на усмотрение

### Step 4 — Адресация в договоре

Для каждого **Critical** и **Major** риска — конкретная формулировка-защита:

| Риск | Mitigation в договоре |
|---|---|
| Неоплата | Pre-payment, milestones, payment via escrow |
| Сорванные сроки | Penalty per day + cure period |
| Качество не то | Acceptance criteria, refund/redo right |
| Уход ключевого человека | Key person clause |
| Confidentiality нарушение | NDA с penalty + termination right |
| Sanctions | Representation + warranty + termination right |
| Tax переквалификация (для самозанятых) | Independent contractor language |

### Step 5 — Risk Register документ

Свести всё в табличку, которая прикладывается к draft договора:

```
| # | Risk | Category | Likelihood | Impact | Score | Mitigation in contract | Status |
|---|------|----------|-----------|--------|-------|----------------------|--------|
| 1 | Просрочка платежа > 30 дней | Financial | High | High | CRITICAL | § 4.3: penalty 0.1%/день, § 8: termination right | Addressed |
| 2 | Раскрытие коммерческой тайны | Reputational | Medium | High | CRITICAL | § 9: NDA + penalty 50,000 EUR | Addressed |
| 3 | Cross-border data transfer | Regulatory | High | Medium | MAJOR | § 11: SCC + DPA | Addressed |
| ... |
```

### Step 6 — Назначить verification date

90 дней после signing — Legal автоматически проверяет: возникли ли проблемы из идентифицированных рисков? Если возник риск, который **не был** в карте — это методологическая ошибка (упустили), обновляется методология.

### Step 7 — Сохранить через `saveResult()`

```javascript
const dept = require('../../../lib/legal');
await dept.saveResult({
  shortTitle: `Risk map: ${dealType} с ${counterparty}`,
  fullResult: formatRiskRegister(register),
  confidence: confidenceLevel,
  jurisdiction: jurisdiction,
  businessForm: businessForm,
  verificationDate: signingDate + 90 days,
});
```

## Worked example

**Контекст:** MSA на разработку ПО между BY ООО (исполнитель, наш клиент) и EU GmbH (заказчик). Сумма 80 000 EUR, срок 6 месяцев.

### Категории риска

**Financial:**
1. Неоплата milestones (Likelihood Medium, Impact High → MAJOR)
2. Curency loss EUR → BYN за 6 месяцев (L:M, I:M → MAJOR)
3. EU customer banking restrictions (L:L, I:H → MAJOR)
4. Налоговая переквалификация при IT-льготе ПВТ (L:L, I:H → MAJOR)

**Operational:**
5. Изменение скоупа заказчиком (scope creep) (L:H, I:M → CRITICAL)
6. Уход lead developer на нашей стороне (L:M, I:H → MAJOR)
7. Acceptance criteria неясны → споры (L:H, I:M → CRITICAL)

**Reputational:**
8. NDA violation (L:L, I:H → MAJOR)
9. Публикация в портфолио без разрешения (L:M, I:L → MINOR)

**Regulatory:**
10. GDPR — если в проекте PII пользователей заказчика (L:M, I:H → CRITICAL)
11. Sanctions — counterparty не в SDN list, but reputation risk (L:L, I:M → MINOR)
12. Cross-border IP rights (L:M, I:M → MAJOR)

### Mitigation в договоре

**Critical:**
- Scope creep → § 3 Change Order procedure + § 4 cap на изменения (max 20% дополнительно с одобрением)
- Acceptance criteria → § 6 detailed AC matrix + § 7 acceptance protocol (15 business days)
- GDPR → § 12 DPA appendix + § 13 SCC for transfer + identify role (controller/processor)

**Major:**
- Неоплата → § 4 milestones (20-30-30-20%) + § 5 penalty 0.05%/день
- Currency risk → § 4.1 cap clause: при отклонении EUR > 5% — пересмотр (с правом отказа)
- EU banking → § 4.2 specifically: Stripe/Wise или прямой банковский перевод; если запрещено — escalation
- Tax переквалификация → § 1 явно "услуги по разработке ПО, не работа", § 2 "Исполнитель действует как ПВТ-резидент BY"
- Lead developer leaving → § 7 Key Person Clause: уведомление за 30 дней + замена с CV для одобрения
- NDA violation → § 9 NDA appendix с penalty 30,000 EUR + termination right
- IP rights → § 10 Work-for-hire после полной оплаты + license back для использования в портфолио (с blur of identifiable info)

**Minor:** оставить как есть или короткие защитные формулировки.

### Verification date — через 90 дней после signing

Legal проверяет:
- Были ли изменения скоупа? Сколько? Сработал ли Change Order процесс?
- Платежи проходили в срок? Penalty применялся?
- Возникли ли вопросы по GDPR?
- Ушёл ли кто-то из ключевых людей?
- Замечания заказчика по качеству?

Любой риск, реализовавшийся **без** mitigation в договоре — методологическая ошибка → обновление anti-patterns в этом скилле.

## Anti-patterns

- **Универсальная карта рисков.** "Вот стандартный список 10 рисков для всех договоров". Это не работа — это шаблон. Каждая сделка уникальна, риски тоже. **Countermeasure:** для каждого договора начинать карту с нуля, проверяя 4 категории под конкретный контекст.

- **Игнорирование позиции владельца.** "В договоре есть penalty 10% — это плохо". Для исполнителя — да. Для заказчика — отлично. **Countermeasure:** всегда фиксировать в Step 1 «наша сторона» и оценивать риски с её точки зрения.

- **Балансирование "справедливости".** "Сделать справедливый договор для обеих сторон". Это работа медиатора, не юриста. Юрист обслуживает свою сторону. **Countermeasure:** честно сказать владельцу: «в договоре нет симметрии, мы защищаем тебя; противная сторона будет требовать симметричных правок — это нормально».

- **Слепое доверие шаблонам.** "У нас есть NDA template, используем его". Шаблон — стартовая точка, не итог. **Countermeasure:** каждый шаблон проверяется через карту рисков для конкретной сделки.

- **Underestimating "minor" risks.** "Это мелочь, можно проигнорировать". Major рисков 5-7, minor 15-20 — суммарно minor могут потопить сделку. **Countermeasure:** все Major/Critical обязательно адресуются; Minor — на усмотрение, но **в письменном виде** (зафиксировано «знаем риск, принимаем»).

- **Risk map без verification.** Карта составлена, договор подписан, верификация не делается. Через год никто не помнит, что было предусмотрено. **Countermeasure:** verification date через 90 дней + 1 год = обязательные шаги.

- **Mitigation в форме «мы потребуем добросовестности»**. Это не mitigation, это надежда. Mitigation = конкретное действие (penalty, termination right, ESCROW, audit right). **Countermeasure:** для каждого риска — действие, а не declaration.

## Output template

```
═══════════════════════════════════════════
RISK REGISTER
═══════════════════════════════════════════

Контекст:
  Сделка: <тип>
  Стороны: <party A>, <jurisdiction A>, <form A> ↔ <party B>, <jurisdiction B>, <form B>
  Наша сторона: <A или B>
  Сумма: <approximate>
  Срок: <duration>
  Критичность: <high/medium/low>

═══════════════════════════════════════════
КРИТИЧЕСКИЕ РИСКИ (обязательно адресовать)
═══════════════════════════════════════════
1. <Risk name>
   Category: <financial/operational/reputational/regulatory>
   Likelihood: <H/M/L>
   Impact: <H/M/L>
   Mitigation: § <X.X> <конкретная формулировка>
2. ...

═══════════════════════════════════════════
ВАЖНЫЕ РИСКИ (рекомендуется адресовать)
═══════════════════════════════════════════
N. <Risk name>
   ...

═══════════════════════════════════════════
МАЛЫЕ РИСКИ (на усмотрение)
═══════════════════════════════════════════
N. <Risk name>
   ...

═══════════════════════════════════════════
Verification date: <YYYY-MM-DD> (90 days after signing)

⚠ Этот документ — risk map + рекомендации. Перед подписанием — обязательная проверка лицензированным юристом.
```

## When NOT to use this skill

- Запрос про **детальный draft конкретного пункта** — это `contract-drafting-discipline`
- Запрос про **анализ чужого договора** на красные флаги — это `red-flag-detection`
- Запрос про **только GDPR** — это `gdpr-compliance-check`

## Integration with Legal

Tier 1, **первый** шаг в любой работе Legal с договором. Все остальные скиллы (drafting, detection, GDPR, IP) применяются **после** того как карта рисков построена, чтобы знать **что именно** искать или формулировать.
