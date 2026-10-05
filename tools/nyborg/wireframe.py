# SPDX-FileCopyrightText: 2026 Nye Warburton
# SPDX-License-Identifier: MIT OR Apache-2.0
# Writes docs/design/nyborg-library-wireframe.svg: the Nyborg library + one Customizer.
# Reuses the yarn-strand Nyborg drawing from gen.py (the approved revision 3 mockup).
import math
src=open('/workspace/blitzwing-check/nyborg/gen.py').read()
src=src.replace("OUT='/workspace/blitzwing/docs/design/nyborg-mockup.svg'","OUT='/tmp/_mock.svg'")
ns={}; exec(src, ns)
nyborg=ns['nyborg']; label=ns['label']; INK=ns['INK']; MUTED=ns['MUTED']; BG=ns['BG']; LINE=ns['LINE']
o_mock=ns['o']; defs=o_mock[o_mock.index('<defs>'):o_mock.index('</defs>')+1]
OUT='/workspace/blitzwing/docs/design/nyborg-library-wireframe.svg'
CARD='#FBF9F5'; EDGE='#E2DCD2'; ACC='#3F6FD8'; SOFT='#EEF3FC'; WARN='#B8742A'
W,H=1200,800
o=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="DejaVu Sans, Helvetica, Arial, sans-serif">',
   '<title>Nyborg library and Customizer wireframe (gate draft)</title>']+defs
def rect(x,y,w,h,fill=CARD,stroke=EDGE,rx=12,extra=''):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" stroke="{stroke}" stroke-width="1.5" {extra}/>'
def button(x,y,w,t,primary=False,h=30):
    f,s,c=(ACC,ACC,'#fff') if primary else ('#fff',EDGE,INK)
    return rect(x,y,w,h,f,s,8)+label(x+w/2,y+h/2+4.5,t,12.5,c,'600')
def chip(x,y,w,t,on=False,h=26):
    f,s,c=(SOFT,ACC,ACC) if on else ('#fff',EDGE,MUTED)
    return rect(x,y,w,h,f,s,13)+label(x+w/2,y+h/2+4.5,t,12,c,'600' if on else '500')
o.append(f'<rect width="{W}" height="{H}" fill="{BG}"/>')
o.append(f'<rect x="12" y="12" width="{W-24}" height="{H-24}" rx="18" fill="none" stroke="{EDGE}" stroke-width="2"/>')
o.append(label(40,52,'Customize — your Nyborgs',22,INK,'700','start'))
o.append(label(40,74,'Wireframe (gate draft). One Customizer for every game. Saved in this browser; export or import as .json.',13,MUTED,'400','start'))

# ---- library list
o.append(rect(30,92,300,684))
o.append(label(50,122,'Library · 4',15,INK,'700','start'))
o.append(button(50,136,80,'+ New',True)); o.append(button(140,136,80,'Import')); o.append(button(230,136,80,'Export all'))
lib=[('Pip','#D9534F',3,'glasses','Tank: Kiter 4-3-2','Racing: Cutter 3-3-3',None),
     ('Juno','#3F6FD8',2,None,'Tank: Charger 4-1-4','Racing: Follower 3-4-2',None),
     ('Tock','#F2C14E',4,'hat','Tank: Sniper 3-1-5','Racing: rebuild needed',WARN),
     ('Ziggy','#C8459A',3,None,'Tank: Gen 99 champion','Racing: Blocker 2-3-4',None)]
for i,(n,c,k,acc,t1,t2,warn) in enumerate(lib):
    y=180+i*96
    sel=i==0
    o.append(rect(42,y,276,86,SOFT if sel else '#fff',ACC if sel else EDGE,10))
    o.append(nyborg(84,y+80,c,k,acc=acc,s=0.66))
    o.append(label(126,y+28,n,15,INK,'700','start'))
    o.append(label(126,y+48,t1,11.5,MUTED,'400','start'))
    o.append(label(126,y+66,t2,11.5,warn or MUTED,'600' if warn else '400','start'))
o.append(label(180,580,'Every Nyborg: same 9-point budget',11.5,MUTED,'400'))
o.append(label(180,597,'per game. Looks are cosmetic only.',11.5,MUTED,'400'))
o.append(f'<line x1="50" x2="310" y1="618" y2="618" stroke="{EDGE}" stroke-width="1.5"/>')
o.append(label(50,644,'Deleted “Rex”.',12.5,INK,'500','start'))
o.append(button(230,626,80,'Undo'))
o.append(label(50,668,'(undo for 10 s)',11,MUTED,'400','start'))

# ---- selected Nyborg header
o.append(label(352,122,'Pip',22,INK,'700','start'))
o.append(label(398,121,'✎ rename',12,ACC,'500','start'))
o.append(button(870,100,92,'Duplicate')); o.append(button(970,100,80,'Export')); o.append(button(1058,100,112,'Delete…'))

# ---- look card
o.append(rect(350,140,290,486))
o.append(label(370,168,'Look',15,INK,'700','start'))
o.append(label(620,168,'cosmetic',11,MUTED,'400','end'))
o.append(f'<ellipse cx="495" cy="300" rx="100" ry="100" fill="#F3EEE6"/>')
o.append(nyborg(495,384,'#D9534F',3,acc='glasses',s=1.85))
o.append(label(370,420,'Hair colour',12.5,INK,'600','start'))
sw=['#D9534F','#B83246','#EE7A68','#3F6FD8','#5AA9E6','#2F4A7A','#F2C14E','#D4A23A','#F08A3C','#C8459A']
for j,c in enumerate(sw):
    cx=382+j*25; cy=442
    if j==0: o.append(f'<circle cx="{cx}" cy="{cy}" r="12" fill="none" stroke="{INK}" stroke-width="2"/>')
    o.append(f'<circle cx="{cx}" cy="{cy}" r="9" fill="{c}" stroke="{LINE}" stroke-width="1.2"/>')
o.append(label(370,484,'Strands',12.5,INK,'600','start'))
for j,n in enumerate(['2','3','4']): o.append(chip(440+j*46,468,40,n,n=='3'))
o.append(label(370,524,'Accessories',12.5,INK,'600','start'))
o.append(chip(370,536,96,'✓ Glasses',True)); o.append(chip(474,536,64,'Hat'))
o.append(label(370,592,'The look is saved with Pip but never',11.5,MUTED,'400','start'))
o.append(label(370,608,'sent to the engine or hashed.',11.5,MUTED,'400','start'))

# ---- build card with game tabs
o.append(rect(656,140,514,486))
o.append(rect(656,140,128,40,'#fff',EDGE,12)); o.append(f'<rect x="657" y="168" width="126" height="13" fill="#fff"/>')
o.append(label(720,166,'Tank Arena',13.5,INK,'700'))
o.append(label(845,166,'Racing',13.5,MUTED,'500'))
o.append(rect(905,148,128,26,'none',EDGE,13,'stroke-dasharray="4 4"')); o.append(label(969,165,'+ next game',12,MUTED,'400'))
o.append(f'<line x1="656" x2="1170" y1="180" y2="180" stroke="{EDGE}" stroke-width="1.5"/>')
o.append(f'<line x1="658" x2="782" y1="180" y2="180" stroke="#fff" stroke-width="3"/>')
o.append(label(676,206,'Build: tabs, stats and numbers come from the engine’s catalog for each game',11.5,MUTED,'400','start'))
# triangle widget
A=(790,236); S=(700,392); D=(880,392)
o.append(f'<path d="M{A[0]},{A[1]} L{S[0]},{S[1]} L{D[0]},{D[1]} Z" fill="#fff" stroke="{LINE}" stroke-width="1.8"/>')
# 19 valid loadout dots: barycentric (a-1,s-1,d-1)/6
for a in range(1,6):
    for s in range(1,6):
        d=9-a-s
        if 1<=d<=5:
            wa,ws,wd=(a-1)/6,(s-1)/6,(d-1)/6
            x=wa*A[0]+ws*S[0]+wd*D[0]; y=wa*A[1]+ws*S[1]+wd*D[1]
            on=(a,s,d)==(4,3,2)
            o.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{7 if on else 3.2}" fill="{ACC if on else "#CFC7BB"}" stroke="{"#fff" if on else "none"}" stroke-width="2"/>')
o.append(label(A[0],A[1]-10,'Attack',12,INK,'600')); o.append(label(S[0]-6,S[1]+20,'Speed',12,INK,'600')); o.append(label(D[0]+6,D[1]+20,'Defense',12,INK,'600'))
# stat rows
stats=[('Attack',4,'24 damage'),('Speed',3,'120 u/s · 45-tick reload'),('Defense',2,'550 HP')]
for j,(n,lv,val) in enumerate(stats):
    y=244+j*48
    o.append(label(920,y,n,13,INK,'600','start'))
    o.append(label(1150,y,f'{lv}',13,INK,'700','end'))
    for p in range(5):
        o.append(rect(920+p*30,y+8,26,10,ACC if p<lv else '#E7E1D7','none',3))
    o.append(label(920,y+34,val,11,MUTED,'400','start'))
o.append(rect(920,388,230,30,SOFT,ACC,8)); o.append(label(1035,408,'9 / 9 points · equal budget',12,ACC,'600'))
o.append(label(676,446,'Presets',12.5,INK,'600','start'))
for j,(n,w) in enumerate([('Balanced 3-3-3',112),('Glass Cannon 5-3-1',140),('Brawler 4-1-4',104),('Scout 2-5-2',96)]):
    xs=[676,794,940,1050][j]; o.append(chip(xs,456,w,n))
o.append(label(676,516,'Behavior',12.5,INK,'600','start'))
o.append(rect(676,528,250,32,'#fff',EDGE,8)); o.append(label(690,549,'Kiter  ·  scripted',12.5,INK,'500','start')); o.append(label(910,549,'▾',12,MUTED,'500','end'))
o.append(label(940,549,'or an evolved champion',11.5,MUTED,'400','start'))
o.append(label(676,598,'Checked by the same validator here, on import and at match start.',11.5,MUTED,'400','start'))

# ---- pick for a match
o.append(rect(350,642,820,134))
o.append(label(370,670,'Pick for a match',15,INK,'700','start'))
o.append(label(370,700,'Tank Arena',12.5,INK,'600','start'))
def slot(x,y,name,color,hair,k,acc=None,empty=False):
    s=rect(x,y,150,52,'#fff',color,10)
    s+=f'<rect x="{x}" y="{y}" width="8" height="52" rx="4" fill="{color}"/>'
    if empty: return s+label(x+80,y+31,name,12,MUTED,'400')
    return s+nyborg(x+36,y+48,hair,k,acc=acc,s=0.44)+label(x+64,y+31,name,13,INK,'600','start')
o.append(slot(370,710,'Pip','#3F6FD8','#D9534F',3,'glasses'))
o.append(label(540,741,'vs',13,MUTED,'600'))
o.append(slot(560,710,'Juno','#F08A3C','#3F6FD8',2))
o.append(button(722,721,84,'Watch ▶',True))
o.append(f'<line x1="826" x2="826" y1="664" y2="760" stroke="{EDGE}" stroke-width="1.5"/>')
o.append(label(846,700,'Racing · cars 1–4 (with M3)',12.5,INK,'600','start'))
for j,(n,c) in enumerate([('Ziggy','#C8459A'),('Pip','#D9534F'),('+',None),('+',None)]):
    x=846+j*78
    o.append(rect(x,710,70,52,'#fff',EDGE,10))
    if c: o.append(nyborg(x+35,758,c,3,s=0.44))
    else: o.append(label(x+35,742,'+ car',12,MUTED,'400'))
o.append('</svg>')
open(OUT,'w').write('\n'.join(o))
print('wrote',OUT)
