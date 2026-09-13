// ── Shared JSON-Schema validation (ADR-0133) ─────────────────────────────
// Наряд №286: валидатор подмножества JSON Schema (ADR-0133: type /
// properties / required / items / enum, строгий отсек лишних полей, громкий
// LLM_SCHEMA_UNSUPPORTED_FEATURE) извлечён из LLM-пути
// (src/builtins/llm_schema.rs, наряд №269) в общий модуль.
//
// Зачем: данные приходят в программу не только от LLM — MCP tool-outputs
// (№268), HTTP-ответы, request_body, поля из БД. Контракт «shape-before-use»
// — проверь форму данных до употребления — требует, чтобы ОДНИ И ТЕМ ЖЕ
// валидатором судились и ответы `call_llm_schema`, и произвольные строки
// через `json_validate`. Дифференциальный контракт: не одного нового
// правила — оба пути зовут один и тот же код (tests/naryad_286_json_validate.rs
// прогоняет общий корпус через оба и сверяет вердикты и тексты нарушений).

pub mod validate;

pub use validate::*;
