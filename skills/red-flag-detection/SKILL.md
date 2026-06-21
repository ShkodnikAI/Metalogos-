---
name: red-flag-detection
description: Выявление опасных пунктов в присланных договорах. Применяется когда контрагент прислал свой draft на подпись и нужно понять что подписать нельзя.
tier: 1
---

# Red Flag Detection — Выявление опасных пунктов

Это **третий главный скилл Legal**. Когда контрагент присылает свой draft договора, твоя задача — за 15-30 минут найти все формулировки, которые при подписании могут стоить тебе денег, прав или нервов. Это **самый частый запрос** в Legal: «вот договор, скажи можно подписывать».

## Prerequisites

- Юрисдикция определена
- Бизнес-форма владельца определена
- Позиция владельца в сделке известна (заказчик/исполнитель/покупатель/продавец)
- Сам текст договора получен

## Core principle

> Опасный пункт договора — это не тот, который **очевидно плохой**. Это тот, который **выглядит нормально**, но при плохом сценарии разрушает позицию. Очевидное другая сторона не пишет; опасное — пишет.

Стандартные шаблоны написаны юристами для **той** стороны. Если контрагент дал тебе **свой** шаблон — он по умолчанию защищает контрагента и не защищает тебя.

## The procedure

### Step 1 — Базовая проверка структуры

Прежде чем искать дьявола в деталях, проверь, что у договора есть **скелет**:
- Полные реквизиты сторон
- Конкретный предмет
- Цена + порядок оплаты
- Срок
- Ответственность
- Применимое право и forum

Если что-то критическое **отсутствует** — это уже red flag (либо ошибка драфтера, либо умысел).

### Step 2 — Прогнать через 12 категорий red flags

#### Category 1 — One-sided penalties

**Признак:** penalty прописан только для **одной** стороны (твоей).

**Пример:** «За нарушение сроков Исполнитель уплачивает 1% за каждый день просрочки». А что с просрочкой оплаты Заказчиком? Молчание = безответственность Заказчика.

**Risk:** Asymmetry. Ты можешь быть оштрафован, контрагент — нет.

**Counter:** Добавить симметричный penalty для другой стороны.

#### Category 2 — Unlimited liability

**Признак:** отсутствие cap (предела) на penalty, indemnity, damages.

**Пример:** «Исполнитель возмещает Заказчику все убытки в полном объёме, включая упущенную выгоду, без ограничения суммы».

**Risk:** Бесконечная ответственность. Один сбой → банкротство.

**Counter:** Cap на совокупную ответственность (обычно = цена договора или 100-200%).

#### Category 3 — Disproportionate penalties

**Признак:** penalty явно завышен относительно нарушения.

**Пример:** «За каждый день просрочки 10% от цены договора». Через 10 дней — 100% цены штрафа.

**Risk:** Один маленький сбой = катастрофические потери.

**Counter:** Снижение % + cap.

#### Category 4 — Vague subject

**Признак:** предмет договора описан размыто, без приложений.

**Пример:** «Исполнитель обязуется оказывать консультационные услуги по запросу Заказчика». Без объёма, конкретики, ТЗ.

**Risk:** Бесконечный объём работ за фиксированную цену. Или наоборот — споры что было предметом.

**Counter:** ТЗ как приложение, конкретные deliverables, change order procedure.

#### Category 5 — Unilateral termination

**Признак:** одна сторона может расторгнуть в любой момент без обоснования.

**Пример:** «Заказчик вправе расторгнуть Договор в любое время с уведомлением за 5 рабочих дней». Исполнитель такого права не имеет.

**Risk:** Заказчик может уйти после получения work product, не оплатив. Или: твои инвестиции в long-term подготовку обесцениваются.

**Counter:** Симметричное право на расторжение + termination for convenience с компенсацией за уже выполненную часть.

#### Category 6 — Choice of unfavorable law/forum

**Признак:** применимое право и forum — в стране/штате контрагента, где у тебя нет ресурсов на защиту.

**Пример:** Делавэр для BY-стороны. Стоимость защиты — десятки тысяч долларов.

**Risk:** Договор подписан, но защититься в споре практически невозможно.

**Counter:** Нейтральная юрисдикция (международный арбитраж в третьей стране — Швеция, Швейцария, Сингапур) или обмен — твой forum за их какое-то условие.

#### Category 7 — Broad confidentiality without exceptions

**Признак:** «вся информация конфиденциальна, нельзя раскрывать ничего». Без exceptions.

**Пример:** «Исполнитель обязуется не раскрывать информацию о сотрудничестве» — не сможешь показать в портфолио.

**Risk:** Невозможность маркетинга работы. Спорно при сделке с известным брендом (а это репутационный актив).

**Counter:** Exceptions: information already public, independently developed, required by law, portfolio use with prior approval.

#### Category 8 — IP grab

**Признак:** все права на «всё созданное в рамках сотрудничества» переходят контрагенту, включая то, что было до договора.

**Пример:** «Все материалы, использованные при выполнении работ, являются собственностью Заказчика».

**Risk:** Теряешь свои pre-existing IP (методологии, библиотеки кода, шаблоны).

**Counter:** Чёткое разделение: pre-existing IP остаётся за тобой; Deliverables — Заказчику после оплаты; общие методики — за тобой.

#### Category 9 — Indemnity for things you don't control

**Признак:** требование indemnify контрагента за то, чем ты не можешь управлять.

**Пример:** «Исполнитель освобождает Заказчика от ответственности перед третьими лицами за претензии к продукту». А если претензия из-за неправильной эксплуатации Заказчиком?

**Risk:** Платишь за чужие ошибки.

**Counter:** Limit indemnity to direct results of your work + carve out для cases when other side caused issue.

#### Category 10 — Change without consent

**Признак:** контрагент может изменить условия в одностороннем порядке.

**Пример:** «Заказчик вправе изменять состав требований к продукту, и Исполнитель обязан выполнять обновлённые требования в том же сроке и за ту же цену».

**Risk:** Бесконечный scope creep, фиксированная оплата.

**Counter:** Change Order procedure — любое изменение требует согласия и может изменить срок/цену.

#### Category 11 — Auto-renewal without explicit consent

**Признак:** договор автопродлевается, отказаться сложно.

**Пример:** «Договор автоматически продлевается на год если не получено уведомление о расторжении за 90 дней до окончания срока». Забыл уведомить — ещё год привязан.

**Risk:** Lock-in. Особенно опасно при подписках, длительных услугах.

**Counter:** Уведомление за разумный срок (14-30 дней) + автоматические напоминания о необходимости решения о пролонгации.

#### Category 12 — Vague payment terms

**Признак:** условия оплаты размыты.

**Пример:** «Заказчик оплачивает работы после приёмки». Срок не указан → 30/60/90 дней? Никто не знает.

**Risk:** Многомесячные задержки оплат «легально».

**Counter:** Конкретные сроки в **рабочих** днях + явное определение «даты оплаты» (списание со счёта Заказчика, а не получение Исполнителем).

### Step 3 — Дополнительные red flags для конкретных типов

#### Для NDA:
- **Mutual vs one-way:** мне нужна mutual NDA (защита моей информации тоже)
- **Срок конфиденциальности > 5 лет** — обычно слишком долго
- **Penalty > $50k** — disproportionate для NDA

#### Для MSA:
- **Право на subcontracting без согласия** — контрагент может перевести всю работу на третью сторону
- **Право на assignment без согласия** — контрагент может передать договор третьей стороне (что особенно опасно при M&A)

#### Для SLA:
- **Метрики SLA измеряются стороной поставщика** — конфликт интересов
- **Penalty за breach SLA < cost мониторинга** — economically не имеет смысла enforcement

#### Для договоров с самозанятыми (со стороны заказчика):
- **Работа выглядит как employment** — formulations с «график», «руководство», «зарплата» — риск переквалификации

### Step 4 — Severity rating каждого red flag

Для каждого найденного red flag:

- **CRITICAL** — must be changed before signing
- **HIGH** — strongly recommend change
- **MEDIUM** — recommend change, can accept with conscious risk
- **LOW** — note for awareness

### Step 5 — Counter-proposals (не просто «нет, плохо», а «вот как должно быть»)

Для каждого CRITICAL и HIGH red flag — конкретная replacement formulation.

```
RED FLAG: § 7.1 "Заказчик вправе расторгнуть Договор в любое время с уведомлением за 5 рабочих дней"
SEVERITY: HIGH
COUNTER: "Каждая Сторона вправе расторгнуть Договор с уведомлением за 30 календарных дней. При расторжении Заказчиком до завершения этапа работ — оплата за фактически выполненную часть."
```

### Step 6 — Risk acceptance log

Если владелец готов принять некоторые red flags (например, потому что сделка стратегически важна) — это **фиксируется письменно** в risk acceptance log, чтобы потом не было «мы не знали».

### Step 7 — Сохранить через `saveResult()`

```javascript
await dept.saveResult({
  shortTitle: `Red flag analysis: ${counterparty} ${dealType}`,
  fullResult: formatRedFlagReport(flags),
  jurisdiction,
  businessForm,
  verificationDate: signingDate + 90 days,
});
```

## Worked example

**Контекст:** Заказчик (US LLC) прислал MSA на разработку. Наша сторона — BY ООО (ПВТ-резидент).

### Red flags найдены

**CRITICAL:**
1. **§ 4.2:** «Customer shall pay invoice within "reasonable time"» — нет конкретного срока
   - Counter: «Customer shall pay invoice within 15 business days from receipt»

2. **§ 7.1:** «Company waives all rights to consequential damages, including but not limited to lost profits, ALL DAMAGES» — too broad, отказ от всех damages включая ущерб от gross negligence
   - Counter: «Each Party waives consequential damages, except for damages caused by gross negligence or willful misconduct»

3. **§ 9.1:** «All IP created in connection with the engagement is the exclusive property of Customer» — включая pre-existing IP
   - Counter: «All NEW IP created specifically for Customer's deliverables is property of Customer upon full payment. Pre-existing IP of Company (libraries, frameworks, methodologies) remains Company's property; Company grants Customer a perpetual non-exclusive license for the deliverables.»

4. **§ 11.1:** «Governing law: Delaware» + «Forum: Delaware courts» — для BY-стороны нерабочее
   - Counter: «Governing law: laws of England and Wales. Forum: LCIA arbitration in London.»

**HIGH:**
5. **§ 5.1:** «Customer may terminate this agreement at any time with 5 days notice; Company may terminate only for cause»
   - Counter: Symmetric termination right with 30 days notice + termination for convenience by either party

6. **§ 8.3:** «Company shall not work with competitors of Customer for a period of 24 months after termination» — non-compete без compensation
   - Counter: Strike, or limit to specifically-named direct competitors with compensation

**MEDIUM:**
7. **§ 3.4:** «Customer may request changes to scope at any time; Company shall implement»
   - Counter: Change Order procedure — any change > 4 hours requires written change order with possible price/timeline adjustment

8. **§ 12.2:** Auto-renewal for 12 months unless cancelled 90 days before
   - Counter: Reduce notice period to 30 days

**LOW:**
9. **§ 14.1:** Notice via email to specific address — risk if address changes
   - Counter: Add backup address + obligation to notify of changes

## Anti-patterns

- **Сразу подписать "потому что договор стандартный".** Никаких "стандартных" договоров для тебя нет — всегда написан под другую сторону. **Countermeasure:** Минимум 1 час review перед signing.

- **Найти "ошибки" но не предложить альтернативу.** "Это плохо" — не работа. Работа = "вот как должно быть". **Countermeasure:** для каждого red flag — counter formulation.

- **Бояться возражать на "стандартный шаблон".** Все договоры negotiable, особенно для повторных клиентов. **Countermeasure:** возражения с конкретными counter — нормальная практика, не "обижает партнёра".

- **Фокус только на больших суммах.** Маленькие договоры часто содержат worst clauses (потому что drafter не тратил на них времени). **Countermeasure:** time-box review по объёму договора, не по сумме.

- **Игнорировать "boilerplate" в конце.** Final provisions (governing law, dispute resolution, notice) — самые недооценённые. **Countermeasure:** обязательно прочитать **каждое** слово в final provisions.

- **"Это можно отрегулировать в процессе".** Нет. Если плохо в договоре — это станет плохо в реальности. **Countermeasure:** все red flags исправлять **до** signing, не "потом разберёмся".

- **Принять risk acceptance без письменной фиксации.** "Я знаю что risk, но подпишу". Через год — "почему ты мне не сказал?" **Countermeasure:** письменный risk acceptance log с подписью владельца.

## Output template

```
═══════════════════════════════════════════
RED FLAG ANALYSIS
═══════════════════════════════════════════

Document: <название, дата, версия>
Counterparty: <name, jurisdiction>
Our side: <position in deal>
Applicable law (текущая redaktion): <law>
Forum: <forum>

═══════════════════════════════════════════
CRITICAL — must change before signing (N штук)
═══════════════════════════════════════════
1. § <X.X>: <quote of problematic clause>
   Risk: <description of risk>
   Counter: <proposed replacement formulation>
2. ...

═══════════════════════════════════════════
HIGH — strongly recommend change (N штук)
═══════════════════════════════════════════
N. ...

═══════════════════════════════════════════
MEDIUM — recommend change (N штук)
═══════════════════════════════════════════
N. ...

═══════════════════════════════════════════
LOW — note for awareness (N штук)
═══════════════════════════════════════════
N. ...

═══════════════════════════════════════════
RECOMMENDATION
═══════════════════════════════════════════
<DO NOT SIGN / SIGN with changes / SIGN with risk acceptance>

If SIGN with risk acceptance:
  Risk Acceptance Log:
  - I, <owner>, acknowledge the following accepted risks: ...
  - Signed: <date>

⚠ Этот анализ — внутренняя оценка. Перед signing — обязательная проверка лицензированным юристом, особенно для договоров суммой > <threshold>.
```

## When NOT to use this skill

- Запрос про **drafting** нового договора — `contract-drafting-discipline`
- Запрос про **risk-mapping** для своей сделки до получения чужого draft — `risk-mapping-protocol`
- Запрос про **GDPR-specific issues** — `gdpr-compliance-check`

## Integration with Legal

Tier 1, применяется при analysis of incoming contracts (чужие drafts). После применения — список red flags + counter-proposals → переговоры с контрагентом.
