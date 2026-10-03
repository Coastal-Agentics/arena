# SPDX-FileCopyrightText: 2026 Nye Warburton
# SPDX-License-Identifier: MIT OR Apache-2.0
# Writes docs/design/nyborg-mockup.svg. Layers (defs): shadow, head, eyes, tuft, glasses, hat.
OUT='/workspace/blitzwing/docs/design/nyborg-mockup.svg'
HEAD='#EADFCB'; LINE='#5E544B'; EYE='#2F2A26'; BG='#F5F2EC'; INK='#5E544B'; MUTED='#8C8378'
TUFTS=[('Moss','#7F9B6B'),('Sage','#A3B48E'),('Sprout','#58C24E'),('Radish','#E2506A')]
def nyborg(x,y,tuft,dy=0,sx=1,sy=1,lean=0,tw=0,acc=None,flip=False,s=1.0,ex=0):
    # cell origin (x,y) = bottom-centre pivot (feet line); head drawn above it
    sh=max(0.55,1-abs(dy)/14)
    g=f'<g transform="translate({x},{y}) scale({s})">'
    g+=f'<ellipse cx="0" cy="0" rx="{22*sh:.1f}" ry="{4.2*sh:.1f}" fill="#000" opacity="0.10"/>'
    inner=f'<g transform="translate(0,{dy}) rotate({lean}) scale({sx},{sy})">'
    inner+=f'<g transform="translate(0,-61) rotate({tw})"><use href="#tuft" fill="{tuft}"/></g>'
    inner+=f'<use href="#head"/><g transform="translate({ex},0)"><use href="#eyes"/>'
    if acc: inner+=f'<use href="#{acc}"/>'
    inner+='</g>'
    inner+='</g>'
    if flip: inner=f'<g transform="scale(-1,1)">{inner}</g>'
    return g+inner+'</g>'
def label(x,y,t,size=13,color=INK,weight='600',anchor='middle'):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" font-weight="{weight}" text-anchor="{anchor}">{t}</text>'
W,H=1000,760
o=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="DejaVu Sans, Helvetica, Arial, sans-serif">',
'<title>Nyborg character mockup (design only)</title>',
'<defs>',
# head: round, sits with its bottom on y=0 (radius 31, centre y=-31)
f'<g id="head"><circle cx="0" cy="-31" r="31" fill="{HEAD}" stroke="{LINE}" stroke-width="2.6"/>'
f'<ellipse cx="-11" cy="-44" rx="9" ry="5" fill="#fff" opacity="0.35" transform="rotate(-25 -11 -44)"/></g>',
f'<g id="eyes" fill="{EYE}"><circle cx="-10" cy="-28" r="3.6"/><circle cx="10" cy="-28" r="3.6"/></g>',
# tuft: drawn around its root (0,0) at the top of the head; fill inherited from <use> (runtime tint)
f'<g id="tuft" stroke="{LINE}" stroke-width="2.2" stroke-linejoin="round">'
'<path d="M0,2 C-6,-4 -15,-8 -19,-17 C-10,-19 -3,-11 0,2 Z"/>'
'<path d="M0,2 C6,-4 15,-7 20,-15 C11,-18 3,-11 0,2 Z"/>'
'<path d="M0,2 C-4,-8 -3,-19 2,-26 C7,-18 5,-8 0,2 Z"/></g>',
f'<g id="glasses" fill="none" stroke="#3B3632" stroke-width="2.2"><circle cx="-10" cy="-28" r="7.5" fill="#fff" fill-opacity="0.25"/>'
'<circle cx="10" cy="-28" r="7.5" fill="#fff" fill-opacity="0.25"/><path d="M-2.5,-29 Q0,-31 2.5,-29"/><path d="M-17.5,-30 L-27,-33 M17.5,-30 L27,-33"/></g>',
f'<g id="hat" stroke="{LINE}" stroke-width="2.2" stroke-linejoin="round" transform="rotate(14 14 -55)">'
'<ellipse cx="14" cy="-54" rx="19" ry="4.5" fill="#5F7A99"/>'
'<path d="M2,-55 C2,-66 7,-71 14,-71 C21,-71 26,-66 26,-55 Z" fill="#6E8BAE"/>'
'<rect x="2.6" y="-60" width="22.8" height="4.4" fill="#E9C46A" stroke="none"/></g>',
'</defs>',
f'<rect width="{W}" height="{H}" fill="{BG}"/>',
f'<rect x="12" y="12" width="{W-24}" height="{H-24}" rx="18" fill="none" stroke="#E2DCD2" stroke-width="2"/>',
label(40,52,'Nyborg — character mockup',22,INK,'700','start'),
label(40,74,'Design only (gate draft). Flat layers: head, eyes, tuft (tinted at runtime), accessory.',13,MUTED,'400','start'),
]
# Row 1: tuft colours + upgrades
o.append(label(40,112,'Tuft colours',15,INK,'700','start'))
o.append(label(690,112,'Upgrades (cosmetic only)',15,INK,'700','start'))
xs=[95,235,375,515]
for (n,c),x in zip(TUFTS,xs):
    o.append(nyborg(x,232,c,s=1.35))
    kind='muted' if n in('Moss','Sage') else 'bright'
    o.append(label(x,262,n)); o.append(label(x,279,f'{c} · {kind}',11,MUTED,'400'))
o.append(nyborg(735,232,TUFTS[0][1],acc='glasses',s=1.35)); o.append(label(735,262,'Glasses')); o.append(label(735,279,'Moss tuft',11,MUTED,'400'))
o.append(nyborg(880,232,TUFTS[3][1],acc='hat',s=1.35)); o.append(label(880,262,'Hat')); o.append(label(880,279,'Radish tuft',11,MUTED,'400'))
o.append(f'<line x1="40" x2="{W-40}" y1="306" y2="306" stroke="#E2DCD2" stroke-width="2"/>')
# Row 2: idle
o.append(label(40,340,'Idle loop — 4 frames at 6 fps (gentle breathing bob)',15,INK,'700','start'))
idle=[(0,1.04,0.96,0),(-1,1,1,3),(-2,0.98,1.03,0),(-1,1,1,-3)]
for i,(dy,sx,sy,tw) in enumerate(idle):
    x=110+i*130
    o.append(f'<rect x="{x-55}" y="356" width="110" height="130" rx="10" fill="#FBF9F5" stroke="#E7E1D7"/>')
    o.append(nyborg(x,466,TUFTS[2][1],dy,sx,sy,0,tw))
    o.append(label(x,504,f'f{i}',12,MUTED,'400'))
o.append(label(640,400,'Cell 112×112 px, pivot at the feet (56, 104).',12,MUTED,'400','start'))
o.append(label(640,420,'Squash on f0, rise on f2;',12,MUTED,'400','start'))
o.append(label(640,440,'the tuft sways ±3°.',12,MUTED,'400','start'))
o.append(f'<line x1="40" x2="{W-40}" y1="524" y2="524" stroke="#E2DCD2" stroke-width="2"/>')
# Row 3: move + flipped
o.append(label(40,556,'Move loop — 6 frames at 10 fps (bounce, lean, tuft wobble; eyes look ahead)',15,INK,'700','start'))
move=[(0,1.06,0.94,7,-6),(-4,1,1,7,-12),(-8,0.97,1.04,6,-4),(-6,1,1,7,6),(-2,1,1,7,11),(0,1.04,0.96,7,3)]
for i,(dy,sx,sy,lean,tw) in enumerate(move):
    x=95+i*118
    o.append(f'<rect x="{x-52}" y="572" width="104" height="132" rx="10" fill="#FBF9F5" stroke="#E7E1D7"/>')
    o.append(nyborg(x,684,TUFTS[3][1],dy,sx,sy,lean,tw,ex=5))
    o.append(label(x,722,f'f{i} →',12,MUTED,'400'))
fx=880
o.append(f'<rect x="{fx-58}" y="572" width="116" height="132" rx="10" fill="#FFF7EC" stroke="#E9B872" stroke-dasharray="5 4"/>')
o.append(nyborg(fx,684,TUFTS[3][1],*move[2][:4],move[2][4],flip=True,ex=5))
o.append(label(fx,722,'← f2, facing left',12,INK,'600'))
o.append(label(fx,738,'(mirrored in x)',11,MUTED,'400'))
o.append('</svg>')
open(OUT,'w').write('\n'.join(o)+'\n')
