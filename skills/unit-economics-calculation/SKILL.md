---
name: unit-economics-calculation
description: Calculate CAC, LTV, payback period correctly with cohort discipline. Use when owner asks about marketing ROI, "is X campaign profitable", "should we invest in customer acquisition", or fundraising prep.
tier: 1
---

# Unit Economics Calculation — расчёт CAC/LTV с дисциплиной когорт

Unit economics — это **финансовая логика одного клиента**. Если она сходится — масштабирование рентабельно. Если нет — масштабирование умножает убыток. Главная сложность не в формулах (они простые), а в том, что **типичные расчёты дают красивые но неверные числа**.

## Prerequisites

- Юрисдикция и бизнес-форма определены (влияет на расходы — налоги, страховые)
- Бизнес-модель определена: SaaS subscription / one-time / freemium / marketplace / другое
- Есть данные за **минимум 6 месяцев** (для cohort analysis)
- Marketing-отдел поставляет расходы на привлечение (если есть Marketing отдел, иначе сам Finance)

## Core principle

> **Unit economics нужно считать по когорте, а не «в среднем за период». Иначе получишь правильные формулы с неправильными числами.**

Классическая ошибка: «у нас CAC = маркетинг бюджет / новые клиенты за месяц». Это **bullshit metric**, потому что:
- Маркетинг в этом месяце привлекает клиентов **этого месяца + следующих** (delay 2-4 недели)
- Новые клиенты этого месяца — частично результат **прошлого маркетинга** (organic, word-of-mouth с lag)
- Если бизнес растёт — числитель и знаменатель не синхронны

**Правильный CAC считается по когорте**: возьми клиентов, пришедших в апреле, и сопоставь с маркетинг-расходами **на привлечение этой когорты** (обычно март + начало апреля + word-of-mouth lag).

Аналогично LTV — это не «средняя выручка с клиента», а **сумма выручки конкретной когорты за её жизненный цикл**, минус variable costs обслуживания.

## The procedure

### Step 1 — Определить cohort (когорту)

Когорта = **группа клиентов, пришедших в один период**.

Период определяется бизнес-моделью:
- **SaaS:** месячные когорты
- **High-touch B2B (ACV >$10k):** квартальные
- **E-commerce / impulse:** недельные
- **Marketplace:** месячные

Для каждой когорты фиксируем:
- Количество клиентов
- Дату начала (первый платёж / первая покупка)
- Channel attribution (откуда пришли)

### Step 2 — Рассчитать CAC (Customer Acquisition Cost)

**Правильная формула:**

```
CAC = (Marketing + Sales costs allocated to cohort) / Number of customers in cohort
```

Что включать в Marketing + Sales costs:
- ✓ Рекламные расходы (ads spend) с правильным attribution
- ✓ Зарплаты sales и marketing команды (proportional to time)
- ✓ Контент-маркетинг (зарплаты, инструменты)
- ✓ Конференции, мероприятия attributed to acquisition
- ✓ Tools (CRM, marketing automation) **частично**
- ✗ **Не включать** общие админ. расходы, аренду, бухгалтерию

**Attribution window:**
- Платная реклама: расходы за **30-90 дней до** когорты (зависит от sales cycle)
- Контент-маркетинг: расходы за **6-12 месяцев до** (контент работает долго)
- Sales team: salary at the time of cohort acquisition + 25% lag

### Step 3 — Рассчитать LTV (Lifetime Value)

**Для SaaS / subscription:**

```
LTV = ARPU × Gross Margin × Customer Lifetime

где:
ARPU (Average Revenue Per User) = выручка с клиента в период / число клиентов
Gross Margin = (Revenue - COGS) / Revenue  (обычно 60-90% для SaaS, 20-40% для e-commerce)
Customer Lifetime = 1 / Monthly Churn Rate (месяцев)
```

Пример: ARPU $50/мес, gross margin 80%, churn 5%/мес → Lifetime = 20 мес → LTV = 50 × 0.80 × 20 = **$800**

**Для one-time purchase:**

```
LTV = Average Order Value × Average Orders Per Customer × Gross Margin
```

**Для marketplace / commission model:**

```
LTV = Average Transaction Value × Take Rate × Transactions Per User × Customer Lifetime
```

### Step 4 — Рассчитать LTV/CAC ratio

```
LTV/CAC ratio = LTV / CAC
```

Интерпретация:
- **< 1.0** — катастрофа, тратишь больше, чем получаешь
- **1.0 – 3.0** — выживание, но без рентабельности
- **3.0 – 5.0** — здоровая зона, можно масштабировать
- **> 5.0** — отлично, **или** под-инвестируешь в маркетинг (не тратишь достаточно)

### Step 5 — Рассчитать CAC Payback Period

```
CAC Payback = CAC / (ARPU × Gross Margin)
```

Через сколько месяцев маркетинг-расходы окупаются gross profit от клиента.

Интерпретация:
- **< 12 месяцев** — отлично
- **12 – 24 месяца** — нормально для B2B SaaS
- **> 24 месяца** — много, нужно либо больше инвестировать (если LTV высокий), либо пересмотреть acquisition

### Step 6 — Корректировки

**По reality-check:**

1. **Churn ≠ self-reported retention.** Многие SaaS считают «retention 90%» через активность, а churn 10% через отписку. Используй **revenue churn**: сколько денег ушло за период.

2. **Discount применяется к LTV.** Будущие $1 не равны сегодняшним. Если discount rate 10%/год — LTV за 5 лет нужно дисконтировать.

3. **Cohort age matters.** Свежая когорта (1-3 месяца) — недостоверная для LTV calculation. Нужно ждать 6+ месяцев минимум.

4. **Segment differences.** Enterprise когорта и SMB когорта — разные unit economics. Не усреднять.

### Step 7 — Verification date

**Обязательно.** Через 90 дней сверить:
- Прогноз LTV с реальным cohort behavior
- Прогноз churn с фактическим
- Прогноз CAC payback с фактическим

Verification date = today + 90 дней. Это позволяет считать metric `unit-economics-accuracy`.

### Step 8 — Сохранить и опубликовать

```js
await finance.saveResult({
  shortTitle: `Unit Economics — <cohort> <jurisdiction>`,
  fullResult: formatUnitEconomicsReport(data),
  confidence: cohortAge < 6 ? 'low' : 'medium',
  verificationDate: today + 90 days,
  jurisdiction,
  businessForm,
});
```

## Worked example

**Сценарий:** SaaS компания (ООО BY), B2B subscription $99/мес

**Когорта апреля 2026:**
- Привлечено 24 новых клиента
- Marketing spend апрель (с lag 30 дней) + 50% марта: $4 800 (Google Ads + content)
- Sales salary attribution: 1 человек × 20% времени × $3 000/мес = $600
- Total acquisition cost: $5 400

**CAC = $5 400 / 24 = $225 за клиента**

**Сейчас (3 месяца после когорты):**
- ARPU: $99/мес (стабилен — все на одном плане)
- COGS (cloud, support time): $15/клиент/мес
- Gross margin: ($99 - $15) / $99 = 85%
- За 3 месяца churn: 2 клиента ушли из 24 = 8.3% за 3 мес = ~2.8%/мес
- Customer lifetime = 1 / 0.028 = ~36 месяцев

**LTV = $99 × 0.85 × 36 = $3 029 за клиента**

**LTV/CAC = $3 029 / $225 = 13.5**

**CAC Payback = $225 / ($99 × 0.85) = $225 / $84.15 = 2.7 месяца**

**Интерпретация:**

- LTV/CAC 13.5 — очень высокий, **возможно under-investing in marketing**
- CAC payback 2.7 мес — отлично для B2B SaaS
- Confidence: **low** — когорта только 3 месяца, lifetime estimate не надёжен

**Recommendations:**

- Можно увеличить marketing budget в 2-3 раза без потери unit economics
- Подождать 6+ месяцев перед окончательным выводом по LTV
- Сегментировать когорту по channel attribution — посмотреть, какие каналы дают лучший LTV

**Output:**

```
📊 Unit Economics — April 2026 cohort (B2B SaaS BY)

Cohort size: 24 customers
Time since cohort start: 3 months
Confidence: LOW (need 6+ months for stable LTV)

— Acquisition —
Marketing spend (attributed): $4 800
Sales attribution: $600
Total CAC cost: $5 400
**CAC: $225 per customer**

— Unit Economics —
ARPU: $99/mo
Gross margin: 85%
Monthly churn: 2.8% (extrapolated from 8.3% over 3 mo)
Lifetime: ~36 months
**LTV: $3 029**

— Ratios —
**LTV/CAC: 13.5** (very high, possibly under-investing)
**CAC Payback: 2.7 months** (excellent)

— Recommendations —
- Increase marketing spend 2-3x without breaking economics
- Wait 6+ months for stable LTV
- Segment by channel attribution

Verification date: <today + 90 days>
```

## Anti-patterns

- **Использовать "average" вместо когорт.** «У нас средний клиент платит $80» — это **бессмысленная** цифра, если клиенты разных возрастов. Когорта обязательна.

- **Включать gross revenue вместо gross margin в LTV.** «LTV = $99 × 36 = $3 564» — это **gross LTV**, не unit economic. Если COGS 15% — net LTV $3 029. Разница есть.

- **Игнорировать sales cycle delay.** Маркетинг апреля привлекает клиентов мая. Если синхронно — занижаешь CAC мая.

- **Считать LTV без discount.** $1 через 5 лет ≠ $1 сегодня. Для длинных lifetimes (>24 мес) нужно применять discount rate.

- **Усреднять сегменты.** Enterprise клиент с CAC $5000 и SMB с CAC $50 в одном расчёте — мешок ерунды.

- **Свежая когорта = надёжный LTV.** Когорта 1-2 месяца не даёт права делать вывод о LTV. Минимум 6 месяцев, лучше 12+.

- **Игнорировать contract length для B2B.** Если у тебя annual contracts — churn нужно мерить через **renewal rate**, а не «отписки». 

- **Учитывать referrals как «бесплатные» клиенты.** Referral не бесплатен — он стоит referral bonus + retention investment в существующих клиентов. Нужно attribute.

## Output template

```
📊 Unit Economics — <cohort label>

**Cohort size:** <N> customers
**Cohort age:** <X> months
**Confidence:** <low (< 6 mo) | medium (6-12 mo) | high (12+ mo)>

— Acquisition Costs —
**CAC:** <X> per customer
Breakdown:
- Marketing spend: ...
- Sales attribution: ...
- Tools: ...

— Lifetime Value —
**ARPU:** <X>/month
**Gross margin:** <X>%
**Monthly churn:** <X>%
**Customer lifetime:** <X> months
**LTV:** <X> per customer

— Ratios —
**LTV/CAC:** <X>
**CAC Payback:** <X> months

— Recommendations —
- ...

Verification date: <today + 90 days>
```

## When NOT to use this skill

- Когда речь о cashflow timing — это `cashflow-forecasting`
- Когда речь о **strategic positioning** маркетинга — это Marketing
- Когда речь о **ROI конкретной кампании** — это Marketing + Finance совместно
- Для **очень молодых** бизнесов (< 6 месяцев) — данных не хватит, оценка ненадёжна

## Integration with finance

Tier 1 скилл. Загружается при упоминании CAC, LTV, unit economics, churn, retention, marketing ROI.

Использует:
- `db.financialForecast` — для исторических когорт
- `db.invoice` — для revenue attribution
- `db.marketingCampaign` (если Marketing отдел активен) — для acquisition spend
- `library/knowledge/finance/jurisdictions/<JX>.md` — для налогов в gross margin

Связь с метрикой: `unit-economics-accuracy` (вторичная метрика) считается через verification.
