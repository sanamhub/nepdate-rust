# Devanagari rendering test strings

Each line exercises a shaping feature. All must render correctly at 14 px and 24 px.

| # | Text | What to check |
|---|---|---|
| 1 | ० १ २ ३ ४ ५ ६ ७ ८ ९ | digits U+0966–U+096F |
| 2 | बैशाख जेठ असार साउन भदौ असोज | month names 1–6 (vowel signs ै, ौ) |
| 3 | कार्तिक मंसिर पुष माघ फागुन चैत | months 7–12: reph र्त, anusvara ं |
| 4 | आइतवार सोमवार मङ्गलवार बुधवार बिहिवार शुक्रवार शनिवार | weekdays: explicit halant ङ्ग, ि before consonant, conjunct क्र |
| 5 | कि की कु कू के कै को कौ कं कः | all matras on क; ि must appear left of क |
| 6 | क्ष ज्ञ श्र त्र द्ध द्व ह्म ट्ट | conjunct ligatures |
| 7 | धर्म कर्म पूर्णिमा चतुर्दशी | reph (र्) above the following consonant |
| 8 | पञ्चमी षष्ठी अष्टमी प्रतिपदा द्वितीया | tithi names (ञ्च, ष्ठ, प्र, द्वि) |
| 9 | नयाँ वर्ष · गणतन्त्र दिवस · बिस्का जात्रा | chandrabindu ँ, conjuncts in events |
| 10 | २०८३ असोज ८ गते, बिहिवार | a full widget header line |
| 11 | Ashoj 8, 2083 · 2026-09-24 · 2:05 PM | Latin fallback in the same line |
