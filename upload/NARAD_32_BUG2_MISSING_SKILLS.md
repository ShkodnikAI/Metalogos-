# НАРЯД #32 — БАГ 2: Отсутствуют скиллы отделов admin и kavalnya

**Приоритет:** HIGH
**Зависимости:** нет (но выполнить ПОСЛЕ #33 если делать вместе)
**Оценка:** 1–2 часа
**Статус:** OPEN

---

## 1. Контекст

Оригинальный баг-наряд говорил что «164 скилла в ZIP-архивах не распакованы
и `skills/fosved/` не существует». Это **частично устарело**:

**Текущее состояние на диске** (`/home/z/my-project/fosved-bot/skills/fosved/`):

| Отдел | На диске | Ожидается (из ZIP) | Статус |
|-------|----------|---------------------|--------|
| design | 12 | 12 | ✅ |
| dev | 20 | 20 | ✅ |
| engineering | 12 | 12 | ✅ |
| expert | 12 | 12 | ✅ |
| finance | 9 | 9 | ✅ |
| kavalna | 0 | 8 | ❌ ПУСТО |
| legal | 10 | 10 | ✅ |
| lz | 12 | 12 | ✅ |
| marketing | 11 | 11 | ✅ |
| qa | 11 | 11 | ✅ |
| spd/osp | 24 | 24 | ✅ (см. NARAD #33) |
| visual | 11 | 11 | ✅ |
| yana | 12 | 12 | ✅ |
| **admin** | **0** | **12** | **❌ ОТСУТСТВУЕТ** |

**Итого:** 144/164 скиллов на месте. Отсутствуют **20 скиллов**:
- `admin/` — 12 скиллов (директория отсутствует)
- `kavalna/` (в ZIP: `kavalnya/`) — 8 скиллов (директория есть, но пуста)

---

## 2. Проблема

1. `verifyIntegrity()` в `departments-registry.js` проверяет `skillsDir` на существование,
   но **не проверяет** количество скиллов внутри. Для `kavalna` директория существует →
   проверка проходит, но внутри 0 скиллов.
2. При маршрутизации на ОСП, Яну и другие отделы — скиллы загружаются
   army.js по путям `fosved/spd/*`, `fosved/yana/*` и т.д. Но скиллы
   admin и kavalnya **недоступны**.
3. В `fosved-all-departments.zip` структура: `fosved-pack/skills/fosved/admin/`
   и `fosved-pack/skills/fosved/kavalnya/`.

---

## 3. Решение

### Шаг 1: Распаковать отсутствующие скиллы из ZIP

```bash
cd /home/z/my-project/fosved-bot

# Создать временную директорию
mkdir -p /tmp/fosved-extract
cd /tmp/fosved-extract

# Распаковать полный архив
unzip -o /home/z/my-project/upload/fosved-all-departments.zip

# admin — 12 скиллов
cp -r fosved-pack/skills/fosved/admin /home/z/my-project/fosved-bot/skills/fosved/

# kavalnya → kavalna (имя директории в ZIP: kavalnya, в боте: kavalna)
cp -r fosved-pack/skills/fosved/kavalnya/* /home/z/my-project/fosved-bot/skills/fosved/kavalna/

# Проверить
echo "admin:" $(ls /home/z/my-project/fosved-bot/skills/fosved/admin/ | wc -l)
echo "kavalna:" $(ls /home/z/my-project/fosved-bot/skills/fosved/kavalna/ | wc -l)
```

Ожидаемый результат:
```
admin: 12
kavalna: 8
```

### Шаг 2: Проверить целостность всех 164 скиллов

```bash
cd /home/z/my-project/fosved-bot
total=0
for d in admin design dev engineering expert finance kavalna legal lz marketing qa spd visual yana; do
  count=$(find skills/fosved/$d -name "SKILL.md" 2>/dev/null | wc -l)
  echo "$d: $count SKILL.md"
  total=$((total + count))
done
echo "---"
echo "TOTAL: $total / 164"
```

### Шаг 3: Усилить verifyIntegrity()

В `lib/departments-registry.js`, функция `verifyIntegrity()` (строка ~266):
добавить проверку минимального количества скиллов.

```javascript
function verifyIntegrity() {
  const issues = [];
  for (const d of DEPARTMENTS) {
    if (d.status !== STATUS.ACTIVE) continue;
    if (d.profile) {
      const p = path.join(PROJECT_ROOT, d.profile);
      if (!fs.existsSync(p)) issues.push({ slug: d.slug, issue: `profile missing: ${d.profile}` });
    }
    if (d.lib) {
      const p = path.join(PROJECT_ROOT, d.lib);
      if (!fs.existsSync(p)) issues.push({ slug: d.slug, issue: `lib missing: ${d.lib}` });
    }
    if (d.skillsDir) {
      const p = path.join(PROJECT_ROOT, d.skillsDir);
      if (!fs.existsSync(p)) {
        issues.push({ slug: d.slug, issue: `skillsDir missing: ${d.skillsDir}` });
      } else {
        // НОВАЯ ПРОВЕРКА: минимум 1 SKILL.md
        const skillCount = fs.readdirSync(p).filter(f => {
          return fs.statSync(path.join(p, f)).isDirectory() &&
                 fs.existsSync(path.join(p, f, 'SKILL.md'));
        }).length;
        if (skillCount === 0) {
          issues.push({ slug: d.slug, issue: `skillsDir empty (0 SKILL.md): ${d.skillsDir}` });
        }
      }
    }
  }
  return { ok: issues.length === 0, issues };
}
```

---

## 4. Файлы для изменения

| Файл | Действие |
|------|----------|
| `skills/fosved/admin/` | Создать директорию с 12 SKILL.md из ZIP |
| `skills/fosved/kavalna/` | Добавить 8 SKILL.md из ZIP (директория есть, но пуста) |
| `lib/departments-registry.js` | Усилить `verifyIntegrity()` — проверять что внутри есть SKILL.md |

---

## 5. Чек-лист выполнения

- [ ] `skills/fosved/admin/` содержит 12 директорий с SKILL.md
- [ ] `skills/fosved/kavalna/` содержит 8 директорий с SKILL.md
- [ ] `find skills/fosved -name SKILL.md | wc -l` = 164
- [ ] `verifyIntegrity()` возвращает `{ ok: true, issues: [] }`
- [ ] Все 164 скилла содержат непустой текст

---

## 6. Примечание о army.js

Файл `army.js` (строка 132+) содержит **хардкод** путей к скиллам:
```javascript
'awareness-frame': 'fosved/spd/awareness-frame/SKILL.md',
// ... 24 записи для spd
```

Если после NARAD #33 (spd → osp) эти пути будут обновлены,
дополнительно нужно убедиться что скиллы admin и kavalna
также зарегистрированы в army.js (если это требуется для их загрузки).

Проверить:
```bash
rg "fosved/admin/" /home/z/my-project/fosved-bot/army.js
rg "fosved/kavalna/" /home/z/my-project/fosved-bot/army.js
```

---

## 7. Откат

```bash
rm -rf /home/z/my-project/fosved-bot/skills/fosved/admin
# Очистить kavalna (оставить только что было ДО):
# Сохранить список файлов ДО выполнения, затем удалить добавленные
```

Вернуть `verifyIntegrity()` к оригинальной версии.

---

## 8. Коммит

```
fix: add missing admin (12) and kavalna (8) department skills

Extracted from fosved-all-departments.zip.
Total skills on disk: 164/164.
Enhanced verifyIntegrity() to check SKILL.md count.

Refs: NARAD #32
```