---
name: cashflow-forecasting
description: Building 30/60/90-day cashflow forecasts with mandatory verification dates. Use when owner asks "how much money will I have", "can I afford X", "what's our runway", or any payment timing question.
tier: 1
---

# Cashflow Forecasting — прогноз cash с обязательной верификацией

Cashflow forecast — это не отчёт «сколько мы заработаем», а **прогноз с verification date**, который через 30/60/90 дней сравнивается с фактом и даёт MAPE метрику для обучения отдела.

## Prerequisites

- Юрисдикция определена (BY / RU / EU / US)
- Бизнес-форма определена (ООО / ИП / самозанятый)
- Подгружены knowledge файлы:
  - `library/knowledge/finance/jurisdictions/<JX>.md`
  - `library/knowledge/finance/business-forms/<FORM>.md`
- Известен текущий cash balance (или его можно получить из БД)
- Известны входящие платежи (инвойсы со статусом не paid)
- Известны исходящие обязательства (налоги, зарплаты, аренда, регулярные расходы)

## Core principle

> **Cashflow forecast — это контракт с будущим, который через 30/60/90 дней проверяется на правдивость.** Без verification date — это не прогноз, а мнение.

В отличие от P&L (который accrual basis — учитывает выручку и расходы в момент возникновения обязательства), cashflow forecast — **только cash basis**: что и когда **реально** поступит на счёт или уйдёт со счёта.

**Главная инверсия интуиции:** прибыльная компания может обанкротиться из-за cashflow. Высокий ARR не спасёт, если deepest pocket клиенты платят с задержкой 90 дней, а зарплата раз в две недели. Cashflow реальнее P&L.

## The procedure

### Step 1 — Установить scope прогноза

Период:
- **Short-term:** 30 дней (1 месяц)
- **Medium-term:** 60-90 дней (квартал)
- **Long-term:** 6-12 месяцев (для бизнеса с runway < 12 месяцев — критично)

Гранулярность:
- 30 дней → **еженедельная** (4 недельных столбца)
- 60-90 дней → **двухнедельная** (6 столбцов)
- 12 месяцев → **ежемесячная** (12 столбцов)

### Step 2 — Собрать opening balance

Это **точная цифра** на день старта прогноза. Не приблизительно:
- Остаток на расчётном счёте (по всем счетам)
- Минус необналиченные платежи (которые еще не пришли)
- Плюс необработанные поступления (которые в processing)

Без точного opening balance — прогноз изначально неверный.

### Step 3 — Спрогнозировать inflows (поступления)

Источники по приоритету достоверности:

1. **Уже выставленные инвойсы со статусом sent/overdue** — высокая достоверность. Применить коэффициент сбора по invoice age:
   - 0-30 дней: 95% expectation
   - 31-60 дней: 70%
   - 61-90 дней: 50%
   - 90+ дней: 30%
   - В юридическом разрешении: 10%
2. **Recurring subscriptions** (MRR/ARR база) — высокая достоверность для существующих подписчиков. Применить **churn rate** (обычно 2-5% месяц).
3. **Контракты с фиксированными датами оплаты** — средняя достоверность. Учесть исторический lag (платят на 5-15 дней позже).
4. **Pipeline (потенциальные сделки)** — низкая достоверность. Применить **probability** (обычно 20-40%).
5. **Прочие источники** (инвестиции, гранты, кредиты) — учитывать только при **подписанном договоре** с конкретной датой.

**Не учитывать в прогнозе:**
- "Возможные клиенты" без переговоров
- Pipeline без вероятности
- "Если выиграем тендер" события

### Step 4 — Спрогнозировать outflows (расходы)

Категории по предсказуемости:

1. **Fixed обязательства** — точные даты, точные суммы:
   - Зарплаты (для ООО): даты по табелю, чистая зарплата + страховые
   - Аренда: дата + сумма по договору
   - Подписки SaaS, инфраструктура: дата + сумма по subscription
   - Кредитные платежи: график погашения
2. **Налоги** — точные даты, расчётная сумма:
   - НДС/VAT: см. налоговый календарь юрисдикции
   - Налог на прибыль / НДФЛ: см. налоговый календарь
   - Социальные взносы: ФСЗН/Insurance
3. **Переменные** — точные категории, оценочная сумма:
   - Маркетинговые расходы (по бюджету маркетинга)
   - Расходы на инфраструктуру (cloud, по предыдущим месяцам)
   - Командировки, обучение
4. **Дискреционные** — события:
   - Подарки клиентам
   - Тимбилдинги
   - Покупка оборудования

### Step 5 — Сделать расчёт по неделям

```
Week 1: opening + inflows W1 - outflows W1 = closing W1
Week 2: closing W1 + inflows W2 - outflows W2 = closing W2
...
Week N: ...
```

Закрытие каждой недели — это **прогноз остатка** на конец недели.

### Step 6 — Найти точки риска

После расчёта — **проверка на отрицательные позиции**:

- Если closing какой-либо недели **< 0** — это **kassal risk** (cash gap)
- Если closing < 10% месячных outflows — это **liquidity risk**
- Если closing постоянно снижается без сезонного объяснения — это **burning risk**

При обнаружении риска — раздел **«Recommendations»** в результате:
- Перенос outflows (если возможно)
- Ускорение inflows (collection campaign)
- Использование credit line / займа
- Сокращение расходов

### Step 7 — Назначить verification date

**Обязательный шаг.** Прогноз без verification date невалиден.

Verification dates:
- 30-day forecast → verify в день +30
- 60-day forecast → verify в день +60 (и +30 как intermediate)
- 90-day forecast → verify в день +30, +60, +90

В Prisma поле `verificationDate` устанавливается на **последний день прогнозного периода**.

### Step 8 — Сохранить через `saveResult()`

```js
await finance.saveResult({
  shortTitle: `Cashflow forecast 30/60/90 (${jurisdiction}/${businessForm})`,
  fullResult: formatForecast(forecast),
  confidence: 'medium', // обычно для cashflow
  verificationDate: calculateVerificationDate(period),
  jurisdiction,
  businessForm,
});
```

Это автоматически публикует прогноз в Telegram archive channel.

### Step 9 — Сформулировать ответ владельцу

Ответ должен содержать:
- Period prognosis (30/60/90 дней)
- Closing balance каждой недели/месяца
- Identified risks (если есть)
- Recommendations (если есть)
- Verification date
- Ссылка на архив

Тон — **сухой, числовой**, без хеджирования. «Через 30 дней remaining cash будет X BYN при условии 70% сбора overdue инвойсов».

## Worked example

**Запрос владельца:** "Hello, кеш потоки на ближайший месяц"

**Контекст:**
- ООО в BY
- Текущий cash: 45 000 BYN на основном счёте + 8 200 USD = ~71 000 BYN total
- 3 outstanding invoices: 12 000 BYN (15 дней), 25 000 USD (40 дней), 8 000 EUR (5 дней)
- Subscription clients: 8 × 500 USD/month = 4 000 USD/month
- Зарплата: 6 человек × 2 500 BYN = 15 000 BYN, выплата 15-го числа
- ФСЗН на зарплату: 15 000 × 28% = 4 200 BYN, до 22-го
- НДС за прошлый месяц: 6 800 BYN, до 22-го
- Аренда: 3 500 BYN, до 5-го
- Cloud: 850 USD/month, 12-го

**Курсы NBRB на сегодня:** USD/BYN = 3.20, EUR/BYN = 3.45

**Решение по неделям (рабочие дни):**

| Неделя | Inflows | Outflows | Closing |
|---|---|---|---|
| Opening | — | — | 71 000 BYN |
| W1 (1-7) | 8000 EUR × 0.95 × 3.45 = 26 220 BYN; subs 1000 USD × 3.20 = 3 200 BYN | Аренда 3 500; cloud 850 × 3.20 = 2 720 | 71 000 + 29 420 - 6 220 = 94 200 |
| W2 (8-14) | 12000 BYN × 0.95 = 11 400; subs 1000 USD = 3 200 | Зарплата (15-го) - вынесем в W3 | 94 200 + 14 600 = 108 800 |
| W3 (15-21) | subs 1 000 USD = 3 200 | ЗП 15 000; ФСЗН 4 200 | 108 800 + 3 200 - 19 200 = 92 800 |
| W4 (22-30) | 25000 USD × 0.70 × 3.20 = 56 000 BYN; subs 1 000 USD = 3 200 | НДС 6 800 | 92 800 + 59 200 - 6 800 = 145 200 BYN |

**Closing 30-day:** ~145 200 BYN

**Risks:**
- W3 closing 92 800 — низкий, но безопасный
- 25 000 USD инвойс (W4) — у возраста 40 дней. Применили коэффициент 70%. Если клиент задержит ещё неделю — будет cash gap в начале следующего месяца.

**Recommendations:**
- Collection call по 25 000 USD инвойсу за неделю до ожидаемой оплаты
- Не делать крупных дискреционных расходов в W3

**Verification date:** +30 дней (конец прогнозного периода) — сверить closing с фактом

**Output:**

```
📊 Cashflow Forecast — ООО BY, 30 days

Opening: 71 000 BYN
W1 closing: 94 200 BYN
W2 closing: 108 800 BYN
W3 closing: 92 800 BYN ⚠ low point
W4 closing: 145 200 BYN

⚠ Risk: 25 000 USD invoice (40 days old) — рекомендую collection call в W3.

Verification date: <today+30>
Confidence: medium

📎 В архиве: <link>
```

## Anti-patterns

- **Прогноз без verification date.** «Через месяц у нас будет 150 000 BYN» без даты проверки — это просто гипотеза. Через месяц никто не вернётся проверить. Без verification обучение отдела невозможно — MAPE не считается.

- **Optimistic invoice collection.** Default коэффициенты сбора по age (95% / 70% / 50% / 30% / 10%) — это **средние** по индустрии. Если ты историски собираешь хуже — корректируй вниз. Никогда не считай overdue 60+ дней как 100%.

- **Игнорирование валютной разницы.** Если у тебя 8 000 USD на счёте, и через месяц прогнозируется получение ещё 25 000 USD — закрытие в BYN зависит от курса на дату операции, не сегодня. Использовать конссервативную оценку курса для outflows (хуже для тебя) и осторожную для inflows.

- **Смешивание cash и accrual.** «Мы выставили инвойс на 50 000 — это уже моя выручка». Нет. До получения денег — это accounts receivable, не cash. Cashflow forecast только cash.

- **Включение pipeline без probability.** «Мы наверное закроем сделку с клиентом X в этом месяце» без подписанного договора — это **не входит в cashflow**. Включается только в P&L pipeline view.

- **Pretending про налоги.** Налоги — fixed obligations. Они не исчезают, если cash low. Их нельзя «отложить» в forecast'е. Если налогов мало — это анти-flag (значит выручка задекларирована меньше).

- **Закрытие на самом важном дне.** Если зарплата 15-го, не делай прогноз с границей W3 = 14-15 числа — это размазывает событие. Зарплату всегда учитывать в неделе, где она реально выплачивается.

## Output template

```
📊 Cashflow Forecast — <BusinessForm> <Jurisdiction>, <N> days

**Opening balance:** <X> <currency>
**Period:** <start> → <end>

| Week | Inflows | Outflows | Closing |
|------|---------|----------|---------|
| W1   | ...     | ...      | ...     |
| ...  | ...     | ...      | ...     |

**Final closing:** <X> <currency>

**Identified risks:**
- ...

**Recommendations:**
- ...

**Verification date:** <YYYY-MM-DD>
**Confidence:** low | medium | high

📎 В архиве: <archive_link>
```

## When NOT to use this skill

- Когда запрос **не про cash**, а про P&L (прибыль, маржу) — это `revenue-recognition-discipline`
- Когда нужно **посчитать unit economics** (CAC/LTV) — это `unit-economics-calculation`
- Когда нужен **strategic forecast > 12 месяцев** — это уровень ОСП, не Finance

## Integration with finance

Это Tier 1 скилл — загружается **всегда** при обращении к Finance. Это базовая методология, которая применяется в большинстве запросов.

Использует данные из:
- `db.invoice` — outstanding invoices
- `db.financialForecast` — prior forecasts (для verification baseline)
- `library/knowledge/finance/jurisdictions/<JX>.md` — налоговый календарь
- `library/knowledge/finance/business-forms/<FORM>.md` — особенности формы

Метрика отдела: `mape-cashflow` рассчитывается через сравнение `verifiedAt` записей с прогнозом.
