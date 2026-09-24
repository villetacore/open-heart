# Новые жители и оформление — 21.09.2026

Встроенный image_gen создал два новых прозрачных атласа. PNG сохранены без перекодирования; область каждого спрайта задаётся в данных/коде.

## Новые ресурсы

- `game/assets/sprites/characters/citizens_atlas.png` — 1536×1024, Сайлас (портной), Рен (чайная), Мика (курьер), три взрослых мужчины-фембоя. Используются NPC в городе и портретами в диалогах. Статические спрайты, не анимации ходьбы.
- `game/assets/decor/femboy_quarter/props_atlas.png` — 1536×1024: стол портного, чайный киоск, скамейка, стойка одежды, фонарь, посылки. В городе размещены десять экземпляров. Шесть вывесок/декораций из партии 02 также подключены. Это декоративные плоские спрайты без коллизии.

## Что подключено

Главное меню использует иллюстрацию с героями. Панель перков — 18 отдельных иконок, HUD — иконку текущего оружия. Диалоги получили портрет и прокручиваемый список ответов. Гардероб показывает иллюстрацию костюмов и 12 иконок аксессуаров; выбранный аксессуар отображается эмблемой в HUD и сохраняется в обычной кампании. Он пока не меняет модель или руки героя.

Сайлас находится около (-5, -2), Рен — (9, -2), Мика — (2, 16), координаты X/Z. Добавлены пять заданий: знакомство с медиком, сбор трёх новых неоновых осколков после принятия задания, прохождение первой глубины, чай для соседа и уничтожение трёх кровавых гончих после принятия. Есть новые места появления осколков и группа гончих на западной стороне города. Это существующий вид врага, новый боевой эпизод. Всего в пресете 25 заданий, 8 цепочек и 15 NPC. Реплики и задания имеют русский и английский текст.

Первые три аксессуара доступны сразу, весь гардероб — после задания портного на материалы. Диалоги дают направления и раскрывают жизнь квартала; награды нельзя получать повторно. Журнал показывает прогресс и готовность задания к сдаче.

## Креатив

Кнопка «Креатив» в главном меню запускает отдельную сессию; F2 открывает панель. Доступны все восемь видов оружия, предметы, ресурсы, аксессуары, переключение классов/специализаций и глубин 1–8. Здоровье, боеприпасы и ресурсы восполняются, способности без ожидания. Перки можно открыть кнопкой панели. Креатив не подключается к серверу, не загружает и не перезаписывает сохранение кампании; сама сессия креатива не сохраняется. Это режим свободного исследования и боя, пока без строительства и полёта.

## Проверка

- Rust: 29 клиентских и 104 теста ядра прошли; clippy с запретом предупреждений.
- Go: `go test ./...`, совпадение хеша пресета с Rust.
- Godot: `runtime_smoke.gd` — все 25 заданий, 8 цепочек, четыре глубины кампании.
- `content_smoke.gd -- --creative` — NPC, декор, цепочка портного, гардероб, повторная награда, неуязвимость, ресурсы, переключение класса/глубины и сохранность файла кампании.
- `content_presentation.gd -- --creative` — снимки реального меню, квартала, диалогов, гардероба, перков и креатива в `target/content-review`.

Проверки движка запускать с отдельным APPDATA внутри target, чтобы не использовать пользовательские сохранения. Релизный экспорт и полное ручное прохождение ещё не выполнены. Это интегрированная партия контента, не заявление о готовности всего проекта к выпуску.

## Промпты и происхождение

Режим: встроенный image_gen, новая генерация, без reference images.

### Жители

Исходник: `exec-ee7a0c19-427d-4c25-979d-ede69885e6be.png`.

GAME CHARACTER SPRITE ATLAS for OpenHeart neon gothic action RPG. Exactly THREE full-body adult male NPCs in THREE EQUAL VERTICAL COLUMNS on a 1536x1024 landscape canvas. TRUE TRANSPARENT background, no scenery, no floor, no shadows cast on background, no text or labels. Each character completely contained in x column of512px with at least60px side margin, head at y90 and boot soles at y960. Same scale, feet level, front three-quarter neutral relaxed standing pose for a game billboard. Hand-painted stylized game sprite, strong readable shapes, crisp silhouette, moderate large details rather than tiny ornamental noise. All are clearly adults aged28-35 and feminine men with flat chests, confident individual characters, tasteful makeup, gothic tailored clothing. LEFT SILAS THE TAILOR: auburn shoulder-length wavy hair, a warm self-assured smile, small heart ear stud, burgundy silk blouse with lace cuffs, black tailored waistcoat, black pleated knee-length skirt over opaque tights, sensible lace-up boots, a measuring tape draped around neck, silver scissors tucked safely into belt, both arms naturally within silhouette. CENTER REN THE TEA-HOUSE KEEPER: dark navy short wavy hair, medium brown skin, round spectacles, cream high-collar shirt with lilac ribbon bow, deep violet waistcoat and dark ankle-length fitted trousers, embroidered dark apron, flat black shoes, holds a small tea tray at waist. RIGHT MIKA THE COURIER: short silver hair with cyan streak, lively adult male face, black short jacket with teal shoulder sash, black tailored shorts over opaque dark leggings, practical knee-high boots, messenger satchel with heart clasp, small rolled map held at side, elegant fingerless gloves. Match a romantic dark gothic city inhabited by adult femboys, with expressive faces and professions. No weapons, no erotic pose, no children, no breasts, no extra limbs. Essential: true transparent empty gutters, all three figures separated, no elements crossing columns.

### Декорации

Исходник: `exec-664f9c09-9d68-4439-b84e-1d870f1883e4.png`.

Production PROP SPRITE ATLAS for OpenHeart neon gothic city, cozy shops of adult femboy inhabitants, six isolated environmental props. Exactly THREE COLUMNS by TWO ROWS on1536x1024, each object completely within its512x512 cell with40px padding. TRUE transparent alpha background, no scene/background/floor, no text. Stylized hand-painted game props, dark wood and worn black metal, restrained rose/cyan/violet accents, clear silhouette suitable for 2.5D billboards. Ground-level three-quarter frontal view with minimal visible top, consistent perspective and lighting. TOP ROW: 1 tailor worktable with sewing machine, neatly folded burgundy cloth and spool of pink ribbon, all one compact unit; 2 little gothic tea kiosk counter with violet canopy, kettle, three teacups and small heart emblem, no person; 3 dark iron bench with rose cushions and a small built-in planter of pink flowers at one end. BOTTOM ROW: 4 freestanding clothing display rack holding a dark pleated skirt and a fitted cyan waistcoat with ribbon, silver heart finials; 5 tall slender black street lamp with heart-shaped pink glass lantern and lilac silk bow, base included; 6 stack of three worn courier crates with rolled fabric, one small heart seal, a strap and silver corners. No letters labels or watermark. Six reusable objects, separate generous transparent gutters, bottom contact points level within each row, detailed but not cluttered.


Дополнительно исправлена развёртка текстуры круглой площади: цилиндры используют мировую triplanar-проекцию вместо масштаба обхват/высота. Проверено на скриншоте квартала; плитка больше не вытянута в полосы.
