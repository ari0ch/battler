"""All 18 roster characters, facing right, idle pose. See engine.py for op syntax."""

PLATE = {"1": "L", "2": "M", "3": "S", "4": "L", "5": "M", "6": "S"}
MAIL_BOOTS = {"1": "L", "2": "M", "3": "S", "4": "C", "5": "B", "6": "b"}
LEATHER = {"1": "C", "2": "B", "3": "b", "4": "B", "5": "b", "6": "b"}


def bow_arc(cols, y0, wood="C", tip="K"):
    ops = []
    for i, c in enumerate(cols):
        y = y0 + i
        mid = tip if i in (0, len(cols) - 1) else wood
        ops.append(f"{y} {c-1} K{mid}K")
    return ops


CHARACTERS = {}

# ───────────────────────────── KNIGHTS ─────────────────────────────
CHARACTERS["bram_gatewarden"] = [
    ("legs", "std", 11, 25, MAIL_BOOTS),
    # short sword held behind
    "9 6 KK", "R 10 17 5 KLMK", "10 5 KWMK",
    # kettle helm + mail coif
    "4 13 KKKKKK", "5 12 KLLLLMSK", "6 11 KLWLLLMMSK",
    "7 9 KKLLLLLLLMMSKK", "8 9 KSSSSSSSSSSSSK", "9 10 KKKKKKKKKKKK",
    "10 11 KMSSPPPPPK", "11 11 KMSpPPKPPPK", "12 11 KMSpPPPPPK",
    "13 11 KMSSpppkK", "14 12 KMSSSSK",
    # body: mail + leather tabard
    "15 10 KKMMSSSSKK", "16 9 KMLMSBBBBB", "17 9 KLMSBCBBBB",
    "18 3 KGGGGGGK", "18 11 KBBCBBB",
    "19 4 KCBBBMSK", "19 12 BBBBBB",
    "20 4 KBBbbKKK", "20 12 BCBBBB",
    "21 5 KGGK", "21 11 KBBBBBBB",
    "22 6 KK", "22 11 KbGbbbbb",
    "23 11 KBBBBBBBB", "24 11 KBBbBBBBB",
    # round wooden shield, iron rim and boss
    "15 21 KKKKK",
    "16 19 KMMMMMSSK",
    "17 19 KMcccCBSK",
    "18 18 KMcccCCCBSK",
    "19 18 KMccLLMCBSK",
    "20 18 KMcCLWMBBSK",
    "21 18 KMCCMMSBbSK",
    "22 18 KMCCBBBBbSK",
    "23 19 KSBBBBbSK",
    "24 19 KSSbbbSSK",
    "25 21 KKKKK",
]

CHARACTERS["sir_aldric"] = [
    ("legs", "std", 11, 25, PLATE),
    # spear behind
    "2 8 KK", "3 7 KLMK", "4 6 KLLMSK", "5 6 KWLMSK", "6 7 KMSK", "7 7 KGgK",
    "R 8 28 7 KCBK", "29 7 KKKK",
    "16 6 KMLMSK", "17 6 KSMSSK",
    # plume
    "2 13 KKK", "3 11 KKRRRK", "4 10 KRRRrrK", "5 9 KRrrK", "6 9 KrK",
    # great helm
    "5 13 KKKKKKK", "6 12 KLLLLLMSK", "7 12 KLWLLLMSK", "8 12 KLLLLLMSK",
    "9 12 KMLLVVVVK", "10 12 KMLLLLMSK", "11 12 KSMLLMVSK", "12 12 KSMMMMSSK",
    "13 12 KKSSSSSKK",
    # plate body
    "14 10 KKMLLLMKK",
    "15 10 KLLMMLMSK", "16 10 KLMMLMSSK", "17 10 KMMLMSSSK",
    "18 10 KMLMSSSSK", "19 10 KSMSSSSSK", "20 10 KGGGGGGGK",
    "21 10 KMLMMSSSK", "22 10 KLMMSSSSK", "23 10 KMMSSSSSK", "24 11 KSSSSSSK",
    # tower shield
    "12 17 KKKKKKKKKK",
    "13 17 KLLLLLLMSK",
    "R 14 26 17 KLCCCGCBSK",
    "17 17 KLGGGGGGSK",
    "27 17 KLBBBgBbSK",
    "28 17 KMSSSSSSSK",
    "29 17 KKKKKKKKKK",
]

CHARACTERS["ysolde_the_unbroken"] = [
    # cape behind
    "14 9 KAAK", "15 8 KaAvK", "16 7 KaAAvK", "17 6 KaAAAvK", "18 6 KaAAAvK",
    "19 5 KaAAAvvK", "20 5 KaAAvvvK", "21 5 KAAAvvvK", "22 4 KaAAAvvK",
    "23 4 KaAAvvvK", "24 4 KAAAvvvK", "25 4 KAAvvvvK", "26 3 KaAAvvvK",
    "27 3 KAAvvvvK", "28 3 KKvKKvKK",
    ("legs", "std", 11, 25, PLATE),
    # raised longsword
    "2 5 KK", "R 3 12 4 KLMK", "3 4 KWMK",
    "13 2 KGYGGYGK", "14 3 KLLMSK", "15 3 KMMSSK", "16 4 KRK",
    "14 8 KMSK", "15 8 KMSK",
    # hair
    "9 9 KYGK", "10 8 KYGgK", "11 8 KGGgK", "12 8 KGgK", "13 8 KGgK", "14 9 KgK",
    # winged helm, open face
    "3 9 KK", "4 8 KWLK", "5 8 KLLMK", "6 9 KLMMK", "7 10 KMSK",
    "5 13 KKKKKK", "6 12 KLLLLLMSK", "7 12 KLWLLLMSK", "8 12 KGGGGGGGK",
    "9 12 KMSpPPPPK", "10 12 KMSpPKPPK", "11 12 KMSpPPPPPK", "12 12 KMSppPpkK",
    "13 13 KSKkkkK",
    # gilded plate
    "14 11 KLLLMMSK", "15 11 KLGLLMSK", "16 11 KLLGLMSK", "17 11 KMLLGMSK",
    "18 11 KMMLLGSK", "19 11 KSMMMSSK", "20 11 KGGGGGGK",
    "21 11 KMLLMMSK", "22 11 KLLMMSSK", "23 11 KMMSSSSK", "24 12 KKKKKK",
    # kite shield with sunburst
    "14 17 KKKKKKKKK", "15 17 KLwwwwxSK", "16 17 KLwwGwxSK", "17 17 KLwGYGxSK",
    "18 17 KLGYWYGSK", "19 17 KLwGYGxSK", "20 17 KLwwGwxSK", "21 17 KLwwwwxSK",
    "22 17 .KLwwxSK.", "23 17 .KLwwxSK.", "24 17 ..KLxSK..", "25 17 ..KLxSK..",
    "26 17 ...KSK...", "27 17 ....K....",
]

# ───────────────────────────── RANGERS ─────────────────────────────
CHARACTERS["pip_quickfeather"] = [
    # quiver
    "12 8 KWK", "13 7 KWwWK", "14 7 KBBBK", "R 15 22 7 KCBbK", "23 7 KKKKK",
    ("legs", "slim", 11, 26, LEATHER),
    # feather + cap
    "7 10 KK", "8 9 KWK", "9 9 KwK", "10 10 KxK",
    "10 12 KKKKKK", "11 11 KNHHHNnK", "12 10 KNHHHHNNnK", "13 10 KKKKKKKKKKK",
    # face, curly hair
    "14 11 KCCPPPPK", "15 11 KCcpPKPPK", "16 11 KCcpPPPPPK", "17 11 KCCppPpkK",
    "18 12 KKkkkKK",
    # tunic
    "19 11 KNNHHNnK", "20 10 KNHHHNNnK", "21 10 KNHHNNNnK", "22 10 KBBGBBBbK",
    "23 10 KNHNNNNnK", "24 10 KNNNNNnnK", "25 11 KKKKKKK",
    # shortbow
    "L 21 14 21 25 E",
    *bow_arc([22, 22, 23, 24, 24, 25, 25, 25, 25, 25, 24, 24, 23, 22], 13),
    "19 19 KNNNKPPK", "20 19 KNNKpkK",
]

CHARACTERS["kestrel"] = [
    # cloak
    "13 10 KnK", "14 9 KnNK", "15 8 KnNNK", "16 8 KnNNK",
    "R 17 25 7 KnNNNK", "26 7 KnNnK", "27 8 KKK",
    ("legs", "slim", 11, 26, LEATHER),
    # longbow (drawn)
    *bow_arc([21, 22, 23, 23, 24, 24, 25, 25, 25, 26, 26, 26, 26, 26, 26, 26,
              25, 25, 25, 24, 24, 23, 23, 22, 21], 3),
    "14 25 KgK", "15 25 KgK", "16 25 KgK",
    "L 21 4 17 12 E", "L 17 12 21 26 E",
    # hood + scarf mask
    "4 13 KKKK", "5 11 KKNNNNK", "6 10 KnNHHHNNK", "7 9 KnNHHNNNNK",
    "8 9 KnNNKPPPPK", "9 9 KnNKpPKPPK", "10 9 KnNKeEEEEK", "11 9 KnNKeeeeEK",
    "12 10 KnNKdddK", "13 10 KnnKKKK",
    # leather body
    "14 10 KBBCCBBK", "15 10 KBCCCBbK", "16 10 KBbCCBbK", "17 10 KBCbCBbK",
    "18 10 KBCCbBbK", "19 10 KBCCBbbK", "20 10 KbbgbbbK",
    "21 10 KnNNNnnK", "22 10 KnNNNnnK", "23 10 KnNNnnnK", "24 10 KnnnnnnK", "25 11 KKKKKK",
    # bow arm + drawing hand + arrow
    "14 17 KBBBBBBK", "15 17 KbbbbbbK",
    "14 24 KPPK", "15 24 KpPK",
    "12 17 CCCCCCCCCC", "11 27 K", "12 27 LK", "13 27 K",
    "11 16 KPPK", "12 16 KPp", "13 11 KBBBBK",
]

CHARACTERS["elowen_starshot"] = [
    # cloak
    "13 9 KNK", "14 8 KNnK", "15 7 KNNnK", "R 16 27 6 KNNnnK", "28 6 KKnKK",
    # long white hair
    "7 10 KwK", "8 9 KwwK", "9 9 KwxK", "R 10 20 9 KwxK", "21 10 KxK",
    ("legs", "slim", 11, 26, {"1": "w", "2": "x", "3": "X", "4": "C", "5": "B"}),
    # recurve bow
    "L 21 6 21 26 E",
    *bow_arc([24, 23, 22, 22, 22, 23, 23, 24, 24, 25, 25, 25, 25, 25, 24, 24,
              23, 23, 22, 22, 22, 23, 24], 5, wood="c", tip="G"),
    "15 24 KgK", "16 24 KgK", "17 24 KgK",
    # star glint
    "2 25 Y", "3 24 YWY", "4 25 Y",
    # head: circlet, pointed ear
    "5 13 KKKKK", "6 12 KwwwwxK", "7 11 KwGGGGGK", "8 11 KwxKPPPPK",
    "9 10 KPkxpPKPPK", "10 11 KxxpPPPPPK", "11 11 KxxppPpkK", "12 12 KxKkkkK",
    # tunic
    "13 12 KNHHNK", "14 11 KNHwHNnK", "15 11 KNHwHNnK", "16 11 KHwGwNnK",
    "17 11 KNHwHNnK", "18 11 KgGgggnK", "19 11 KNHHNNnK", "20 11 KNHNNnnK",
    "21 11 KNNNnnnK", "22 11 KNNnnnnK", "23 11 KHNNnnnK", "24 11 KNNnnnnK", "25 12 KKKKKK",
    # arm to grip
    "16 18 KNNNNKPK", "17 18 KnnnKKpK",
]

# ───────────────────────────── MAGES ─────────────────────────────
CHARACTERS["tobble_the_apprentice"] = [
    # oversized patched hat, leaning back
    "5 11 KK", "6 11 KcK", "7 11 KcCK", "8 11 KcCBK", "9 11 KcCCBK",
    "10 11 KcCYCBK", "11 10 KcCYYYBK", "12 10 KcCCYCCBK", "13 10 KcCCCCCBbK",
    "14 7 KcccCCCCCCCCBBbK", "15 7 KKKKKKKKKKKKKKKKK",
    # face
    "16 12 KkPPPKPK", "17 12 KkPPPPPPK", "18 13 KkppkK",
    # robe (too long for him)
    "19 11 KaAAAAvK", "20 10 KaAAAAAvK", "21 10 KaAAAAAvK", "22 10 KaAAAAvvK",
    "23 10 KcccccccbK", "24 10 KaAAAAAvK", "25 9 KaAAAAAAvK", "26 9 KaAAAAAAvvK",
    "27 9 KaAAAAAAvvK", "28 9 KaAAAAAAvvK", "29 9 KAAAAAAvvvK", "30 9 KKKKKKKKKKKK",
    "31 11 KbbbKbbbK",
    # wand + spark
    "L 22 20 25 16 B",
    "12 26 F", "13 25 FYF", "14 24 FYWYF", "15 25 FYF", "16 26 F",
    "20 18 KaAK", "19 21 KPPK", "20 21 KpkK",
]

CHARACTERS["mirelle_frostweaver"] = [
    # long pale hair down the back
    "R 12 20 9 KJIiK", "21 10 KiK",
    # robe
    "12 12 KJJJIK", "13 11 KJJJIIiK", "14 11 KJwJIIiK", "15 11 KJJwIIiK",
    "16 11 KJJIwIiK", "17 11 KJJIIIiK", "18 11 KiiiiiiK", "19 11 KJJIIIiK",
    "20 10 KJJJIIIiK", "21 10 KJJIIIIiK", "22 10 KJJIIIiiK", "23 9 KJJJIIIiiK",
    "24 9 KJJIIIIiiK", "25 9 KJJIIIiiiK", "26 8 KJJJIIIiiiK", "27 8 KJJIIIIiiiK",
    "28 8 KJJIIIiiiiK", "29 8 KJIIIIiiiiK", "30 8 KwwwwwwwwwK", "31 8 KKKKKKKKKKK",
    # head + ice tiara
    "3 15 KJK", "4 13 KKIKK", "5 12 KJJJJIK", "6 11 KJWJJJIK", "7 11 KJJIPPPPK",
    "8 11 KJIiPKPPK", "9 11 KJIiPPPPPK", "10 11 KJIipPpkK", "11 11 KJIiKkkK",
    # crystal staff
    "R 8 30 21 KxXK", "31 21 KKKK",
    "2 23 KK", "3 22 KWJK", "4 21 KWJIiK", "5 21 KJJIiK", "6 21 KJIiiK", "7 22 KIiK",
    "3 19 J", "6 26 J", "9 25 I", "12 26 J",
    # arm
    "15 17 KJIIK", "16 17 KIiiK", "15 21 KPPK", "16 21 KpkK",
]

CHARACTERS["archmage_vorn"] = [
    # orb staff behind
    "2 5 KKKK", "3 4 KaWaAK", "4 4 KAaAvK", "5 5 KvvK", "6 5 KCBK",
    "R 7 30 5 KCBK", "31 5 KKKK",
    "15 4 KPPK", "16 4 KpkK", "15 8 KAK", "16 8 KvK",
    # robe
    "13 9 KAAAAvK", "14 9 KaAAAAvvK", "15 9 KaAAAAvvK", "16 9 KaAAAvvvK",
    "17 9 KaAAAAvvK", "18 9 KGGGGGGGK", "19 9 KaAAAAvvK", "20 9 KaAAAvvvK",
    "21 8 KaAAAAAvvK", "22 8 KaAAAAvvvK", "23 8 KaAAAAvvvK", "24 8 KaAAAvvvvK",
    "25 7 KaAAAAAvvvK", "26 7 KaAAAAvvvvK", "27 7 KaAAAAvvvvK", "28 7 KaAAAvvvvvK",
    "29 7 KGGGGGGGGgK", "30 7 KKKKKKKKKKK", "31 9 KbbbKbbbK",
    # hood, old face, huge beard
    "4 13 KKKK", "5 11 KKAAAvK", "6 10 KAaAAAvK", "7 10 KAaAKKKKK",
    "8 10 KAAKEEPPK", "9 10 KAAKpPKPPK", "10 10 KAAKwPPPPPK", "11 10 KAKwwwwwwK",
    "12 10 KAKwWwwwxK", "13 11 KKwwwwxxK", "14 12 KwwwwxxK", "15 12 KwwwxxK",
    "16 13 KwwxxK", "17 13 KwxxK", "18 14 KxK",
    # fireball in hand
    "12 23 F", "13 22 KFYFK", "14 21 KFYWYFK", "15 21 KFYWWYFK", "16 21 KFYYYFK",
    "17 22 KFFFK", "18 22 KPPPK", "19 19 KAAKpkK",
]

# ───────────────────────────── CLERICS ─────────────────────────────
CHARACTERS["sister_agnes"] = [
    # veil down the back
    "R 13 19 8 KdDK",
    # habit
    "14 10 KDDDDddK", "15 10 KDDDDddK", "16 10 KeDDDddK", "17 10 KeDDDddK",
    "18 10 KcccccbK", "19 10 KeDDDddK", "20 9 KeDDDDddK", "21 9 KeDDDdddK",
    "22 9 KeDDDdddK", "23 9 KeDDDdddK", "24 8 KeDDDDdddK", "25 8 KeDDDDdddK",
    "26 8 KeDDDddddK", "27 8 KeDDDddddK", "28 8 KeDDdddddK", "29 8 KeDDdddddK",
    "30 8 KKKKKKKKKKK", "31 10 KbbbKbbbK",
    "19 15 G", "20 14 GGG", "21 15 G",
    # veil + wimple
    "5 12 KKKKKK", "6 11 KDDDDDdK", "7 10 KDDwwwwwK", "8 10 KDwwPPPPwK",
    "9 10 KDwpPPKPwK", "10 10 KDwpPPPPwK", "11 10 KDwwpppwwK", "12 10 KDDwwwwwK",
    "13 9 KDDDwwwxdK",
    # prayer book
    "15 17 KKKKKK", "16 17 KCCCBK", "17 17 KCGCBK", "18 17 KCCCBK", "19 17 KwwwxK",
    "20 17 KKKKKK", "17 16 KP", "18 22 PK", "19 22 pK",
]

CHARACTERS["brother_calloway"] = [
    # lantern staff
    "4 21 KKKKKK", "5 21 KCCCBK", "6 25 K", "7 25 K",
    "8 24 KKK", "9 23 KgYgK", "10 23 KYWYK", "11 23 KgYgK", "12 24 KKK",
    "8 27 Y", "10 28 Y", "13 22 Y",
    "R 6 30 21 KCBK", "31 21 KKKK",
    # robe (broad)
    "14 8 KCCCCBBBBBbK", "15 8 KCCCCBBBBBbbK", "16 8 KCCCBBBBBBbbK", "17 8 KCCCBBGBBBbbK",
    "18 8 KCCBBGGGBBbbK", "19 8 KCCBBBGBBBbbK", "20 8 KcccccccccccK",
    "21 8 KCCBBBBBBBbbK", "22 8 KCCBBBBBBbbbK", "23 8 KCCBBBBBBbbbK",
    "24 7 KCCCBBBBBBbbbK", "25 7 KCCBBBBBBBbbbK", "26 7 KCCBBBBBBbbbbK",
    "27 7 KCCBBBBBBbbbbK", "28 7 KCBBBBBBbbbbbK", "29 7 KCBBBBBBbbbbbK",
    "30 7 KKKKKKKKKKKKKK", "31 9 KbbbbKKbbbbK",
    # tonsured head, short beard
    "5 13 KKKKK", "6 12 KPWPPpK", "7 11 KPPPPPPpK", "8 11 KCPPPPPPK",
    "9 11 KCCpPPKPPK", "10 11 KCCpPPPPPPK", "11 11 KCpPCCCCK", "12 12 KpCCCBK",
    "13 13 KCBBK",
    # hand on staff
    "15 19 KBBK", "16 19 KBbK", "15 21 KPPK", "16 21 KpkK",
]

CHARACTERS["seraphine_dawnbringer"] = [
    # wings
    "9 8 KK", "10 6 KKWWK", "11 5 KWWwwK", "12 4 KWwwxwK", "13 4 KWwxwxK",
    "14 3 KWwxwxxK", "15 3 KwwxwxxK", "16 4 KwxwxxK", "17 4 KxwxxK", "18 5 KxxxK", "19 6 KxK",
    # hair down the back
    "R 12 19 9 KGgK",
    # robe
    "12 12 KwwwxK", "13 11 KwwwGwxK", "14 11 KWwwGwxK", "15 11 KwwwGwxK", "16 11 KwwGGGxK",
    "17 11 KwwwGwxK", "18 11 KGGGGGGK", "19 11 KwwwwxxK", "20 10 KWwwwwxxK",
    "21 10 KwwwwwxxK", "22 10 KwwwwxxxK", "23 9 KWwwwwxxxK", "24 9 KwwwwwxxxK",
    "25 9 KwwwwxxxXK", "26 8 KWwwwwxxxXK", "27 8 KwwwwwxxxXK", "28 8 KwwwwxxxXXK",
    "29 8 KwwwwxxxXXK", "30 8 KGGGGGGGGGK", "31 8 KKKKKKKKKKK",
    # halo + head
    "2 12 KYYYYYYK", "3 13 KKKKKK",
    "5 13 KKKKK", "6 12 KYGGGgK", "7 11 KYGGPPPPK", "8 11 KGGgPKPPK",
    "9 11 KGGgPPPPPK", "10 11 KGgpPPpkK", "11 11 KGgKkkkK",
    # sun staff
    "3 22 KKKK", "4 21 KGYYGK", "5 21 KYWWYK", "6 21 KYWWYK", "7 21 KGYYGK", "8 22 KKKK",
    "2 23 YY", "3 20 Y", "3 27 Y", "5 19 Y", "5 28 Y", "8 20 Y", "8 27 Y",
    "R 9 30 22 KGgK", "31 22 KKKK",
    "16 18 KwwK", "17 18 KxxK", "16 21 KPP", "17 21 KpkK",
]

# ───────────────────────────── ROGUES ─────────────────────────────
CHARACTERS["nettle"] = [
    # crouched legs
    "22 9 KBCBK", "23 8 KBCBK", "24 8 KBBK", "R 25 27 8 KBbK", "28 8 KbbK",
    "29 8 KbbbK", "30 8 KbbbbK", "31 8 KKKKKK",
    "22 14 KBCCBK", "23 16 KBCCBK", "24 18 KBCBK", "R 25 27 18 KBbK", "28 18 KbbK",
    "29 18 KbbbK", "30 18 KbbbbK", "31 18 KKKKKK",
    # torso
    "17 10 KBCCBBBK", "18 10 KBCCBBbK", "19 10 KbbgbbbK", "20 10 KBCCBbbK", "21 11 KBBBbK",
    # hood + bandana
    "9 13 KKKK", "10 11 KKBCCBK", "11 10 KBCCCCBK", "12 10 KBCCKKKKK",
    "13 10 KBCKpPKPK", "14 10 KBCKNNNNNK", "15 10 KBBKnNNNK", "16 11 KBBKKKK",
    # knife
    "18 17 KBBKPPK", "19 17 KBKKpkK",
    "17 22 g", "18 22 g", "19 22 g", "17 23 KKKK", "18 23 LLWK", "19 23 KKKK",
]

CHARACTERS["vex"] = [
    # scarf
    "12 8 KKAAAK", "13 5 KKAAAAvK", "14 4 KAAvvK", "15 3 KAvK", "16 3 KK",
    ("legs", "slim", 12, 26, {"1": "D", "2": "d", "3": "d", "4": "d", "5": "V"}),
    "R 22 25 12 KDdKDdK",
    # back dagger (reverse grip)
    "16 9 KDDK", "17 8 KDDK", "18 8 KDdK", "19 8 KPPK", "20 7 KgggK",
    "21 8 KLK", "22 8 KLK", "23 8 KLK", "24 8 KK",
    # body
    "13 12 KDDDDdK", "14 11 KDeDDDdK", "15 11 KDeDDddK", "16 11 KDDvDddK",
    "17 11 KDDDvddK", "18 11 KDDDDvdK", "19 11 KgdddddK", "20 11 KDDDDddK", "21 11 KDDDdddK",
    # spiky hair + domino mask
    "4 11 KK", "4 15 KK", "3 11 K", "3 16 K",
    "5 10 KaAKKaAK", "6 10 KaaAAaAAK", "7 11 KaAAvAAvK",
    "8 11 KAvAPPPPK", "9 11 KAvdddWdK", "10 11 KAvPPPPPPK", "11 11 KAvpPpkK", "12 12 KKkkkK",
    # front dagger
    "16 18 KDDDPPK", "17 18 KKKKKKK",
    "15 24 gKKK", "16 24 gLLWK", "17 24 gKKK",
]

CHARACTERS["the_grey_widow"] = [
    # cloak
    "11 10 KEeeeeDDK", "12 9 KEeeeeeDDK", "13 9 KEeeeeeDDK", "14 9 KEeeeeeDDK",
    "15 8 KEeeeeeDDdK", "16 8 KEeeeDeeDdK", "17 8 KEeeeDeeDdK", "18 8 KEeeeDeeDdK",
    "19 8 KEeeeDeeDdK", "20 7 KEeeeeDeeDDK", "21 7 KEeeeeDeeDdK", "22 7 KEeeeeDeeDdK",
    "23 7 KEeeeeDeeDdK", "24 6 KEeeeeeDeeDdK", "25 6 KEeeeeeDeeDdK", "26 6 KEeeeeeDeeDdK",
    "27 6 KEeeeeeDeeDdK", "28 5 KEeeeeeeDeeDdK", "29 5 KEeeeeeeDeeDdK",
    "30 5 KEeKeeKeeKeeDK", "31 5 KKKKKKKKKKKKKK",
    # hood with veiled face + glowing eyes
    "2 14 KK", "3 13 KEeK", "4 12 KEeeDK", "5 11 KEeeeDDK", "6 11 KEeeKKKDK",
    "7 10 KEeeKVVVVK", "8 10 KEeeKVaVaK", "9 10 KEeeKVVVVK", "10 10 KEeeeKKKDK",
    # spider brooch
    "13 14 KdK", "14 13 KdadK", "15 14 KdK",
    # curved blades
    "17 19 KddK", "16 21 KLK", "15 22 KLK", "14 23 KLK", "13 24 KLWK", "12 25 KLK", "11 25 KK",
    "20 18 KddK", "20 21 g", "21 21 KLK", "22 22 KLK", "23 23 KLK", "24 23 KLWK", "25 24 KK",
    "16 21 g",
]

# ───────────────────────────── BERSERKERS ─────────────────────────────
CHARACTERS["grub_ironjaw"] = [
    ("legs", "stout", 9, 26, {"1": "C", "2": "B", "3": "b", "4": "B", "5": "b", "6": "b"}),
    # back arm
    "16 6 KqOK", "17 5 KqOOK", "18 5 KOOoK", "19 5 KOoK", "20 4 KOOoK", "21 5 KKK",
    # body
    "15 8 KqOOOK", "15 18 KOOoK",
    "16 8 KqOOOOOObOOoK", "17 8 KOOOOOObOOOoK", "18 8 KOOOOObOOOooK", "19 8 KoOOObOOOOooK",
    "20 8 KooObOOOOoooK", "21 9 KbbbbgbbbbbK", "22 9 KBCCCCBBBbK", "23 9 KBCCCBBBBbK",
    "24 9 KBCCBBBBbbK", "25 9 KBBBBBBbbbK",
    # head: topknot, iron jaw, tusk
    "6 14 KK", "7 13 KbbK", "8 12 KqOOOoK", "9 11 KqOOOOOoK", "10 11 KqOOObbbK",
    "11 11 KOOOoYKOOK", "12 11 KOoLLLLLLK", "13 11 KoKLWLMwSK", "14 11 KoKMMMMSSK",
    "15 12 KKSSSSKK",
    # crude axe
    "R 9 24 22 KCBK", "25 22 KKKK",
    "8 25 KKK", "9 25 KMLK", "10 25 KMLLK", "11 25 KSMLK", "12 25 KSMLK", "13 25 KSMMK", "14 25 KKKK",
    "17 20 KqOOK", "18 20 KOooK",
]

CHARACTERS["hilda_bloodaxe"] = [
    # double-bit axe on her back
    "R 4 27 6 KCBK", "28 6 KKKK",
    "3 3 KKK", "4 2 KWLK", "5 2 KLMSK", "6 2 KLMSK", "7 2 KLMSK", "8 2 KLMK", "9 3 KMK", "10 3 KK",
    "3 10 KKK", "4 10 KLWK", "5 9 KSMLK", "6 9 KSMLK", "7 9 KSMLK", "8 10 KMLK", "9 10 KMK", "10 10 KK",
    ("legs", "std", 11, 25, {"1": "C", "2": "B", "3": "b", "4": "c", "5": "C", "6": "B"}),
    # braid down the back
    "12 11 KRK", "13 11 KrK", "14 11 KRK", "15 11 KrK", "16 11 KRK", "17 11 KK",
    # horned helm + face
    "3 12 KK", "4 12 KwK", "5 13 KwK", "3 21 KK", "4 20 KwK", "5 19 KwK",
    "5 14 KKKKK", "6 13 KLLLMSK", "7 13 KMMMMMMK",
    "8 13 KRrPPKPK", "9 13 KRrpPPPPK", "10 13 KRrpPpkK", "11 13 KRrKkkK",
    # fur mantle
    "12 12 KcCcCcCcK", "13 10 KccCcCcCcxK", "14 10 KxcxcxcxcxK",
    # mail + leather
    "15 11 KMLMMMMSK", "16 11 KLMMMMSSK", "17 11 KMMMMSSSK", "18 11 KBBgBBBbK",
    "19 11 KCCBBBbbK", "20 11 KCBBBBbbK", "21 11 KCBBBbbbK", "22 11 KBBBbbbbK",
    "23 11 KBBbbbbbK", "24 12 KbbbbbK",
    # front braid + fist
    "13 19 KRK", "14 19 KrK", "15 19 KRK", "16 19 KK",
    "16 20 KPPK", "17 20 KpkK",
]

CHARACTERS["korgath_the_undying"] = [
    ("legs", "stout", 9, 26, {"1": "D", "2": "d", "3": "V", "4": "B", "5": "b", "6": "b"}),
    # greataxe
    "R 4 30 23 KCBK", "31 23 KKKK",
    "3 24 KKKK", "4 24 KLWLK", "5 24 KLLLMK", "6 24 KMLLMSK", "7 24 KMLMSSK", "8 24 KMMSSSK",
    "9 24 KMSSSK", "10 24 KSSSK", "11 24 KKKK",
    "4 22 KM", "5 22 KS",
    # back arm
    "14 5 KEeK", "15 4 KEeeK", "16 4 KEeDK", "17 4 KeeDK", "18 4 KeDDK", "19 4 KeDK", "20 4 KKK",
    # huge torso, scars, chain
    "12 8 KEEeeeeK", "13 7 KEEEeeeeDK", "14 7 KEEeeeeDDDK", "15 8 KEeerreeDDK",
    "16 8 KEeeereeDDK", "17 8 KEeeeereDDK", "18 8 KMSMSMSMSMK", "19 8 KeeeeeeDDdK",
    "20 9 KeeeeeDDdK", "21 9 KbbbgbbbbK", "22 9 KDDDDddddK", "23 9 KDDDdddddK",
    "24 9 KDDdddddVK", "25 9 KDdddddVVK",
    # skull pauldron
    "11 16 KKKKK", "12 15 KwwwwxK", "13 15 KwKwKxK", "14 15 KwwwxxK", "15 16 KxKxK",
    # head: mohawk, heavy brow, glowing eye
    "3 13 KKKK", "4 12 KddddK", "5 11 KEEEEeeK", "6 11 KEEEeeeeK", "7 11 KEeeddddK",
    "8 11 KEeeeeYeK", "9 11 KEereeeeeeK", "10 11 KeerDDDDK", "11 12 KeDDDDK",
    # front fist on haft
    "15 21 KEeK", "16 21 KEeeK", "17 21 KeeDK",
]

OFFSET = {"hilda_bloodaxe": 2, "korgath_the_undying": -1}

ORDER = [
    "bram_gatewarden", "sir_aldric", "ysolde_the_unbroken",
    "pip_quickfeather", "kestrel", "elowen_starshot",
    "tobble_the_apprentice", "mirelle_frostweaver", "archmage_vorn",
    "sister_agnes", "brother_calloway", "seraphine_dawnbringer",
    "nettle", "vex", "the_grey_widow",
    "grub_ironjaw", "hilda_bloodaxe", "korgath_the_undying",
]
