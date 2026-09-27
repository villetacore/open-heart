"""Author the Shared Table episode. Re-runnable; only owns evening_* IDs.

After running, sync quests/dialogues RON with the sync_preset Rust example.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PRESET = ROOT / 'game/presets/core'
ART = 'res://assets/illustrations/neighbors/'

def loc(ru, en):
    return dict(ru=ru, en=en)

def line(speaker, portrait, ru, en):
    return dict(speaker=loc(*speaker), portrait=portrait, text=loc(ru, en))

def choice(ru, en, **kwargs):
    return dict(text=loc(ru, en), **kwargs)

names = dict(ash=('Эш', 'Ash'), noel=('Ноэль', 'Noel'), lucien=('Люсьен', 'Lucien'),
             emil=('Эмиль', 'Emil'), ivo=('Иво', 'Ivo'), yves=('Ив', 'Yves'))

quests = json.loads((PRESET / 'quests.json').read_text(encoding='utf-8'))
dialogues = json.loads((PRESET / 'dialogues.json').read_text(encoding='utf-8'))
quests = [q for q in quests if not q['id'].startswith('evening_')]
dialogues = [s for s in dialogues if not s['id'].startswith('evening_')]

# Distinct events already emitted by the running game. Counts start on acceptance.
specs = [
    dict(id='evening_circuit', giver='ash', title=loc('Свет без мерцания', 'A Steady Light'),
         kind='solve_puzzle', target='*', count=1, requires=['neighbors_ash_2'], xp=180, gold=70,
         desc=loc('После принятия синхронизируй одну группу из трёх реле в комнате головоломки на глубине 2 или ниже. Вернись к Эш на южную дорогу. Старые решения не засчитываются.',
                  'After accepting, synchronize one group of three relays in a puzzle room at depth 2 or deeper. Return to Ash on the southern road. Earlier solutions do not count.'),
         offer=[('Образцы помогли подобрать лампы. Но питание всё ещё скачет. Для общего вечера нужна схема, которая выдержит больше одного фонаря.',
                 'The samples helped us choose lamps, but the supply is still unstable. Our shared evening needs a circuit that can support more than one lantern.'),
                ('Внизу сохранились релейные группы. Синхронизируй одну по порядку и запомни последовательность. Я повторю её на нашей линии — вытаскивать из стены провода не надо.',
                 'Relay groups survive below. Synchronize one in order and remember the sequence. I can reproduce it here; there is no need to pull wires out of the walls.')],
         progress=('Ищи комнату с тремя реле со второго яруса. Ошибка сбрасывает порядок, но не закрывает попытку. Мне нужен устойчивый цикл, а не самая быстрая попытка.',
                   'Look for a room with three relays from depth two onward. A mistake resets the sequence, but you can retry. I need a stable cycle, not a speed record.'),
         complete=[('Теперь понимаю, где терялся импульс. Лампы перестанут спорить друг с другом. Я закончу монтаж, а ты загляни к Ноэлю: он ищет историю для вечера.',
                    'Now I see where the pulse was getting lost. The lamps can stop arguing with one another. I will finish the wiring; visit Noel, who is looking for a story for the evening.')]),
    dict(id='evening_memory', giver='noel', title=loc('Не только имена победителей', 'More Than Victors’ Names'),
         kind='discover_lore', target='*', count=1, requires=['evening_circuit', 'neighbors_noel_2'], xp=180, gold=70,
         desc=loc('После принятия найди и прослушай одно эхо памяти в сюжетной комнате на глубине 3 или ниже. Вернись к Ноэлю. Уже услышанные до принятия записи не засчитываются.',
                  'After accepting, listen to one memory echo in a story room at depth 3 or deeper. Return to Noel. Echoes heard before acceptance do not count.'),
         offer=[('Для вечера все предлагают рассказ о победе. Я хочу оставить место тем, кто жил здесь между победами.',
                 'Everyone suggests a victory story for the evening. I want room for the people who lived here between victories.'),
                ('На третьем ярусе встречаются эхо памяти. Послушай одно целиком. Принеси не легенду, а то, что действительно услышишь. Даже если там будет только чей-то страх.',
                 'Memory echoes appear from depth three onward. Listen to one in full. Bring back what you actually hear, not a legend, even if all it contains is someone’s fear.')],
         progress=('Эхо ищи в тихой сюжетной комнате, не среди реле. Коснись источника памяти. Не нужно приписывать голосу имя, если он сам его не назвал.',
                   'Look for an echo in a quiet story room, not among the relays. Interact with the memory source. Do not assign the voice a name it never gave.'),
         complete=[('Запишу твой пересказ как свидетельство, не как окончательную правду. Сегодня за столом можно говорить и о том, чего мы пока не понимаем.',
                    'I will record your account as testimony, not a final truth. Tonight there is room at the table for things we do not yet understand.'),
                   ('Люсьен готовит музыку. Кажется, он боится нарушить тишину больше, чем я — заговорить. Спроси, не нужна ли ему помощь.',
                    'Lucien is preparing the music. I think he fears breaking the silence more than I fear speaking. Ask whether he needs a hand.')]),
    dict(id='evening_music', giver='lucien', title=loc('Тише, чем обычно', 'Softer Than Usual'),
         kind='interact', target='yves', count=1, requires=['evening_memory', 'neighbors_lucien_2'], xp=90, gold=35,
         desc=loc('После принятия поговори с Ивом в южной части квартала о тихом месте рядом с двором. Вернись к Люсьену.',
                  'After accepting, speak with Yves in the southern quarter about a quiet place beside the courtyard. Return to Lucien.'),
         offer=[('Я могу сыграть громко. Хочу понять, умею ли сыграть так, чтобы никто не чувствовал себя запертым на концерте.',
                 'I can play loudly. I want to know whether I can play without making anyone feel trapped at a concert.'),
                ('Поговори с Ивом. У сада нужен свободный угол, куда можно отойти с чашкой. Это будет частью вечера, а не местом для тех, кому «не понравилось».',
                 'Talk to Yves. We need a quiet corner by the garden where someone can take their cup. It should be part of the evening, not a place for people who “didn’t like it”.')],
         progress=('Ив у южной дороги. Спроси о тихом уголке и возвращайся. Пока подберу мелодию, которая не пытается заполнить собой весь двор.',
                   'Yves is by the southern road. Ask about the quiet corner and come back. I will find a melody that does not try to fill the entire courtyard.'),
         complete=[('Оставим проход свободным. Первую сыграю тихо, а потом послушаю разговоры. Теперь к Эмилю: музыка без ужина слишком похожа на репетицию.',
                    'We will leave the path clear. I will play the first tune softly, then listen to the conversation. See Emil next; music without dinner feels too much like rehearsal.')]),
    dict(id='evening_table', giver='emil', title=loc('Место за столом', 'A Place at the Table'),
         kind='interact', target='ren', count=1, requires=['evening_music', 'neighbors_emil_2'], xp=160, gold=60,
         desc=loc('После принятия пригласи Рена из чайной на площади, затем вернись к Эмилю на южную дорогу. За завершение появится общий стол рядом с пекарем.',
                  'After accepting, invite Ren from the tea house on the square, then return to Emil on the southern road. Completing this quest adds a shared table beside the baker.'),
         offer=[('С дорогой ты уже помог, свет Эш наладила. Осталось не забыть человека, который обычно наливает чай всем остальным.',
                 'You helped with the road; Ash fixed the lights. Now we must remember the person who usually pours tea for everyone else.'),
                ('Загляни к Рену. Сегодня он гость. Чашки у нас есть. Возвращайся вместе с добрыми новостями — хлеб как раз успеет остыть.',
                 'Visit Ren. Tonight he is a guest. We have cups. Come back with the good news; the bread will have cooled by then.')],
         progress=('Рен в чайной на площади. Не проси приносить посуду и не превращай приглашение в новый заказ. Просто скажи, что мы его ждём.',
                   'Ren is at the tea house on the square. Do not ask him to bring dishes or turn the invitation into another order. Just tell him we are expecting him.'),
         complete=[('Значит, ставим ещё одну чашку. Стол готов. Это не награда только для победителя — это ужин для всех, кто дожил до вечера.',
                    'Then we will set out another cup. The table is ready. This is not a prize just for the victor; it is dinner for everyone who made it to evening.')])
]

for stage, spec in enumerate(specs, 1):
    qid, giver = spec['id'], spec['giver']
    q = dict(id=qid, title_ru=spec['title']['ru'], title_en=spec['title']['en'],
             desc_ru=spec['desc']['ru'], desc_en=spec['desc']['en'], giver=giver,
             chain_ru='Свет для общего стола', chain_en='Light for the Shared Table', stage=stage,
             kind=spec['kind'], target=spec['target'], count=spec['count'], requires=spec['requires'],
             reward_xp=spec['xp'], reward_gold=spec['gold'],
             reward_items=[dict(id='bread' if stage == 4 else 'medkit', qty=2 if stage == 4 else 1)])
    if stage == 4:
        q['world_change'] = dict(id='shared_table', pattern='table', pos=[-11.0,0.0,43.0],
                                 color=[1.0,0.66,0.36], scale=1.0, sprite='', activity='crowd',
                                 activity_count=2, activity_radius=2.5, activity_speed=0.3)
    for state in ('offer','progress','complete'):
        q[state+'_scene'] = qid+'_'+state
    quests.append(q)
    for state in ('offer','progress','complete'):
        paragraphs = [spec['progress']] if state == 'progress' else spec[state]
        lines = [line(names[giver], giver, *p) for p in paragraphs]
        if state == 'offer':
            lines.append(line(names[giver], ART+giver+'.tres', spec['desc']['ru'], spec['desc']['en']))
            options = [choice('Я помогу.', 'I’ll help.', effects=[dict(kind='quest', id=qid, title=spec['title'], desc=spec['desc'])]),
                       choice('Вернусь позже.', 'I’ll return later.')]
        elif state == 'progress':
            options = [choice('Продолжу.', 'I’ll keep going.')]
        else:
            effects = [dict(kind='quest_done',id=qid),dict(kind='xp',value=spec['xp']),dict(kind='gold',value=spec['gold'])]
            if stage == 4:
                effects.append(dict(kind='flag',flag='shared_evening'))
            options = [choice('Завершить задание.', 'Complete quest.', effects=effects,
                              **({'next':'evening_epilogue'} if stage == 4 else {}))]
        dialogues.append(dict(id=q[state+'_scene'],lines=lines,choices=options))

finale = [
 ('ash','Видишь? Ни мерцания. Я почти забыла, как выглядит просто свет.', 'See? No flicker. I had almost forgotten what ordinary light looks like.'),
 ('emil','Тарелок хватит. А если придёт ещё кто-то, подвинемся. Из-за лишнего гостя никого домой не отправляют.', 'There are enough plates. If someone else comes, we will make room. Nobody gets sent home for being one guest too many.'),
 ('ivo','Цветы без сильного запаха, как просил Рен. Впервые за неделю мне не хочется ничего поправлять в букете.', 'Unscented flowers, as Ren asked. For the first time this week, I do not want to adjust the bouquet.'),
 ('yves','У сада оставлен проход. Захочешь тишины — отойди. Место за столом останется твоим.', 'The path by the garden is clear. Step away if you need quiet. Your place at the table will still be yours.'),
 ('lucien','Первую сыграю тихо. Хочу сначала услышать, как вы разговариваете.', 'I will play the first one softly. I want to hear everyone talking first.'),
 ('noel','Дату я записал. Остальное попробую запомнить без бумаги.', 'I have written down the date. I will try to remember the rest without paper.')
]
dialogues.append(dict(id='evening_epilogue',
    lines=[line(names[g],ART+'shared_evening_v1.png',ru,en) for g,ru,en in finale],
    choices=[choice('Останусь на чашку чая.', 'I’ll stay for a cup of tea.'),choice('Ещё вернусь. Берегите друг друга.', 'I’ll be back. Look after one another.')]))

after = {
 'ivo': [('Ноэль попросил сохранить засохший цветок из букета. Не для описи. Просто попросил.', 'Noel asked me to keep a dried flower from the bouquet. Not for an inventory. He simply asked.'),
         ('Наутро я снова открыл холодильник и принял поставку. Хороший вечер не отменяет работу. Зато напоминает, для кого она.', 'The next morning I opened the fridge and received a delivery. A good evening does not cancel work. It reminds you who it is for.')],
 'noel': [('В записи о вечере нет списка заслуг. Есть имена тех, кто пришёл, и пятно от чая.', 'The record of our evening has no list of achievements. It has the names of those who came and a tea stain.'),
          ('Пятно оставлю. Источники иногда сообщают больше, когда их не пытаются привести в идеальный вид.', 'I will keep the stain. Sources sometimes tell you more when you stop trying to make them perfect.')],
 'lucien': [('Я сыграл тише, чем на репетиции. Оказалось, никто не перестаёт слушать только потому, что может свободно разговаривать.', 'I played more softly than at rehearsal. People did not stop listening just because they could talk freely.'),
            ('В следующий раз попрошу кого-нибудь выбрать первую песню. Не хочу, чтобы общий вечер зависел только от моего настроения.', 'Next time I will ask someone else to choose the first song. A shared evening should not depend only on my mood.')],
 'ash': [('Ночью я всё-таки проверила линию ещё раз. Привычка. Но впервые делала это без страха, что сейчас кто-то позовёт из темноты.', 'I checked the circuit once more that night. Habit. For once, I did not expect someone to call from the dark.'),
         ('Если лампы снова погаснут, мы знаем, с чего начать. Надёжность — это ещё и возможность починить всё второй раз.', 'If the lamps go out again, we know where to start. Reliability also means being able to repair something a second time.')],
 'emil': [('Хлеб закончился раньше чая. Считаю это хорошим результатом. Осталось только вернуть Рену его чашку — он всё равно принёс свою.', 'We ran out of bread before tea. I call that a success. Now I must return Ren’s cup; he brought his own anyway.'),
          ('Стол оставим. Ему не обязательно ждать следующего праздника. Завтра за ним можно просто позавтракать.', 'We will keep the table. It need not wait for another celebration. Tomorrow someone can simply have breakfast there.')],
 'yves': [('В тихом уголке кто-то оставил две чашки рядом. Значит, тишину тоже можно разделить.', 'Someone left two cups together in the quiet corner. Silence can be shared too.'),
          ('Я убрал чашки, но не поставил табличку. Пусть это место остаётся приглашением, а не инструкцией.', 'I cleared the cups but put up no sign. Let the place remain an invitation, not an instruction.')]
}
for giver, paragraphs in after.items():
    sid = 'evening_after_'+giver
    dialogues.append(dict(id=sid, lines=[line(names[giver],ART+giver+'.tres',*p) for p in paragraphs],
                          choices=[choice('Вспомним тот вечер.', 'Remember that evening.',next='evening_epilogue'),
                                   choice('Вернёмся к разговору.', 'Back to our conversation.',next='neighbors_'+giver)]))
    intro = next(s for s in dialogues if s['id']=='neighbors_'+giver)
    intro['choices'] = [c for c in intro['choices'] if not str(c.get('next','')).startswith('evening_')]
    intro['choices'].insert(-1, choice('Как тебе общий вечер?', 'How was our shared evening?',
                                       requires=dict(flag='shared_evening'),next=sid))

# NPCs actually respond to the invitation instead of silently advancing counters.
for npc, qid, sid, response in [
    ('yves','evening_memory','evening_yves_corner',('Оставлю проход вдоль клумб. И скамейку подальше от сцены. Передай Люсьену: уходить в тишину — не значит уходить от него.',
                                                 'I will leave a path beside the beds and a bench away from the stage. Tell Lucien that stepping into quiet does not mean stepping away from him.')),
    ('ren','evening_music','evening_ren_invitation',('Мне не нужно ничего приносить? Даже чайник? Тогда я приду. Непривычно слышать приглашение, в котором нет заказа.',
                                                   'I do not need to bring anything? Not even a kettle? Then I will come. An invitation without an order takes some getting used to.'))
]:
    intro_id = 'neighbors_yves' if npc=='yves' else 'ren_intro'
    # Discover Ren's actual named scene from NPC data, not an assumed ID.
    if npc=='ren':
        intro_id = next(n['scene'] for n in json.loads((PRESET/'npcs.json').read_text(encoding='utf-8')) if n['id']=='ren')
    intro = next(s for s in dialogues if s['id']==intro_id)
    intro['choices'] = [c for c in intro['choices'] if c.get('next') != sid]
    intro['choices'].insert(0,choice('Об общем вечере…', 'About our shared evening…',requires=dict(quest_done=qid,not_flag='shared_evening'),next=sid))
    speaker = names['yves'] if npc=='yves' else ('Рен','Ren')
    dialogues.append(dict(id=sid,lines=[line(speaker,npc,*response)],choices=[choice('Передам. Спасибо.', 'I’ll tell them. Thank you.',next=intro_id)]))

for stem, content in [('quests',quests),('dialogues',dialogues)]:
    (PRESET/(stem+'.json')).write_text(json.dumps(content,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'Evening episode: 4 quests, 21 scenes. Total: {len(quests)} quests, {len(dialogues)} scenes.')
