# SPDX-FileCopyrightText: 2026 Nye Warburton
# SPDX-License-Identifier: MIT OR Apache-2.0
# Writes docs/design/nyborg-mockup.svg (revision 3: 2-4 chunky yarn strands, no mouth).
# Layers: head, eyes, hair strands (colour = runtime tint), accessory.
import math
OUT='/workspace/blitzwing/docs/design/nyborg-mockup.svg'
HEAD='#EADFCB'; LINE='#5E544B'; EYE='#2F2A26'; BG='#F5F2EC'; INK='#5E544B'; MUTED='#8C8378'
HAIR=[('Yarn Red','#D9534F','default',3),('Cobalt','#3F6FD8','blue',2),('Sunny','#F2C14E','yellow',4),('Magenta','#C8459A','extra',3)]
# strand: (root x, lean in degrees from straight up, length, curl direction, flop multiplier)
STRANDS={2:[(-5,-22,34,-1,1.0),(6,26,30,1,1.25)],
         3:[(-9,-38,28,-1,1.25),(0,-4,36,1,1.0),(9,36,28,1,1.25)],
         4:[(-13,-50,24,-1,1.3),(-4.5,-20,31,-1,1.0),(4.5,18,31,1,1.0),(13,48,24,1,1.3)]}
def strand_points(x0,lean,L,curl,flop):
    # heading along the strand: lean + a soft gravity droop outward + a gentle wave
    # + trailing sway, and the last third turns into a curl at the tip
    side=1 if lean>0 else -1 if lean<0 else curl
    pts=[(x0,4.0)]; n=48; ds=L/n
    for i in range(n):
        u=(i+0.5)/n
        th=(lean + side*38*u**2 + 11*math.sin(2*math.pi*u)*(-side)
            + flop*1.8*u + curl*190*max(0.0,(u-0.66)/0.34)**1.3)
        th=math.radians(th)
        x,y=pts[-1]; pts.append((x+ds*math.sin(th), y-ds*math.cos(th)))
    return pts
def strand_svg(hair,pts):
    d='M'+' L'.join(f'{x:.2f},{y:.2f}' for x,y in pts)
    out=f'<path d="{d}" fill="none" stroke="{LINE}" stroke-width="10" stroke-linecap="round" stroke-linejoin="round"/>'
    out+=f'<path d="{d}" fill="none" stroke="{hair}" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>'
    # twisted ply: short darker diagonals along the strand
    ticks=''
    for i in range(3,len(pts)-3,3):
        (xa,ya),(xb,yb)=pts[i-1],pts[i+1]
        tx,ty=xb-xa,yb-ya; m=math.hypot(tx,ty) or 1; tx,ty=tx/m,ty/m
        nx,ny=-ty,tx; x,y=pts[i]
        dx,dy=(nx*0.85+tx*0.55)*2.8,(ny*0.85+ty*0.55)*2.8   # ~35 degrees off the normal
        ticks+=f'M{x-dx:.2f},{y-dy:.2f} L{x+dx:.2f},{y+dy:.2f} '
    out+=f'<path d="{ticks}" stroke="#000" stroke-opacity="0.18" stroke-width="1.2" stroke-linecap="round"/>'
    # soft highlight along one side
    hl=[(x-1.3,y-0.6) for x,y in pts[2:int(len(pts)*0.6)]]
    out+='<path d="M'+' L'.join(f'{x:.2f},{y:.2f}' for x,y in hl)+'" fill="none" stroke="#fff" stroke-opacity="0.35" stroke-width="1.4" stroke-linecap="round"/>'
    return out
def hair_svg(hair,count,sway):
    return ''.join(strand_svg(hair,strand_points(x0,lean+sway*0.35,L,c,sway*f)) for x0,lean,L,c,f in STRANDS[count])
def nyborg(x,y,hair,count=3,dy=0,sx=1,sy=1,lean=0,sway=0,acc=None,flip=False,s=1.0,ex=0):
    sh=max(0.55,1-abs(dy)/14)
    g=f'<g transform="translate({x},{y}) scale({s})">'
    g+=f'<ellipse cx="0" cy="0" rx="{22*sh:.1f}" ry="{4.2*sh:.1f}" fill="#000" opacity="0.10"/>'
    inner=f'<g transform="translate(0,{dy}) rotate({lean}) scale({sx},{sy})">'
    inner+='<use href="#head"/>'
    inner+=f'<g transform="translate(0,-62)">{hair_svg(hair,count,sway)}</g>'
    inner+=f'<g transform="translate({ex},0)"><use href="#eyes"/>'
    if acc=='glasses': inner+='<use href="#glasses"/>'
    inner+='</g>'
    if acc=='hat': inner+='<use href="#hat"/>'
    inner+='</g>'
    if flip: inner=f'<g transform="scale(-1,1)">{inner}</g>'
    return g+inner+'</g>'
def label(x,y,t,size=13,color=INK,weight='600',anchor='middle'):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" font-weight="{weight}" text-anchor="{anchor}">{t}</text>'
W,H=1000,780
o=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="DejaVu Sans, Helvetica, Arial, sans-serif">',
'<title>Nyborg character mockup, revision 3 (design only)</title>',
'<defs>',
f'<g id="head"><circle cx="0" cy="-31" r="31" fill="{HEAD}" stroke="{LINE}" stroke-width="2.6"/>'
f'<ellipse cx="-12" cy="-42" rx="8" ry="4.5" fill="#fff" opacity="0.30" transform="rotate(-25 -12 -42)"/></g>',
f'<g id="eyes"><circle cx="-10" cy="-29" r="4" fill="{EYE}"/><circle cx="10" cy="-29" r="4" fill="{EYE}"/>'
'<circle cx="-8.8" cy="-30.4" r="1.2" fill="#fff"/><circle cx="11.2" cy="-30.4" r="1.2" fill="#fff"/></g>',
f'<g id="glasses" fill="none" stroke="#3B3632" stroke-width="2.2"><circle cx="-10" cy="-29" r="7.8" fill="#fff" fill-opacity="0.25"/>'
'<circle cx="10" cy="-29" r="7.8" fill="#fff" fill-opacity="0.25"/><path d="M-2.2,-30 Q0,-32 2.2,-30"/><path d="M-17.8,-31 L-28,-34 M17.8,-31 L28,-34"/></g>',
# hat: sits on the head; strands poke out from under the brim and over the crown
f'<g id="hat" stroke="{LINE}" stroke-width="2.2" stroke-linejoin="round" transform="rotate(8 0 -58)">'
'<ellipse cx="0" cy="-57" rx="24" ry="5" fill="#5F7A99"/>'
'<path d="M-13,-58 C-13,-70 -7,-74 0,-74 C7,-74 13,-70 13,-58 Z" fill="#6E8BAE"/>'
'<rect x="-12.4" y="-63.5" width="24.8" height="4.4" fill="#F5F2EC" stroke="none"/></g>',
'</defs>',
f'<rect width="{W}" height="{H}" fill="{BG}"/>',
f'<rect x="12" y="12" width="{W-24}" height="{H-24}" rx="18" fill="none" stroke="#E2DCD2" stroke-width="2"/>',
label(40,52,'Nyborg — character mockup (revision 3)',22,INK,'700','start'),
label(40,74,'Design only (gate draft). Hair: 2–4 chunky yarn strands. Layers: head, eyes, hair (tinted at runtime), accessory.',13,MUTED,'400','start'),
]
o.append(label(40,108,'Hair colours and strand counts',15,INK,'700','start'))
o.append(label(690,108,'Upgrades (cosmetic only)',15,INK,'700','start'))
xs=[95,235,375,515]
for (n,c,k,cnt),x in zip(HAIR,xs):
    o.append(nyborg(x,256,c,cnt,s=1.35))
    o.append(label(x,284,n)); o.append(label(x,300,f'{c} · {k}',11,MUTED,'400')); o.append(label(x,314,f'{cnt} strands',11,MUTED,'400'))
o.append(nyborg(735,256,HAIR[1][1],2,acc='glasses',s=1.35)); o.append(label(735,284,'Glasses')); o.append(label(735,300,'Cobalt · 2 strands',11,MUTED,'400'))
o.append(nyborg(880,256,HAIR[2][1],3,acc='hat',s=1.35)); o.append(label(880,284,'Hat')); o.append(label(880,300,'Sunny · 3 strands',11,MUTED,'400'))
o.append(f'<line x1="40" x2="{W-40}" y1="330" y2="330" stroke="#E2DCD2" stroke-width="2"/>')
o.append(label(40,352,'Idle loop — 4 frames at 6 fps (gentle bob, strands sway)',15,INK,'700','start'))
idle=[(0,1.04,0.96,0),(-1,1,1,4),(-2,0.98,1.03,0),(-1,1,1,-4)]
for i,(dy,sx,sy,sw) in enumerate(idle):
    x=110+i*130
    o.append(f'<rect x="{x-55}" y="366" width="110" height="136" rx="10" fill="#FBF9F5" stroke="#E7E1D7"/>')
    o.append(nyborg(x,484,HAIR[0][1],3,dy,sx,sy,0,sw))
    o.append(label(x,520,f'f{i}',12,MUTED,'400'))
o.append(label(640,410,'Cell 112×112 px, pivot at the feet (56, 104).',12,MUTED,'400','start'))
o.append(label(640,430,'Squash on f0, rise on f2;',12,MUTED,'400','start'))
o.append(label(640,450,'the strands sway ±4° and lag the head.',12,MUTED,'400','start'))
o.append(f'<line x1="40" x2="{W-40}" y1="540" y2="540" stroke="#E2DCD2" stroke-width="2"/>')
o.append(label(40,572,'Move loop — 6 frames at 10 fps (bounce, lean, strands flop and trail)',15,INK,'700','start'))
# (dy, sx, sy, lean, sway): negative sway = strands trail back (to the left) while moving right
move=[(0,1.06,0.94,7,-10),(-4,1,1,7,-18),(-8,0.97,1.04,6,-12),(-6,1,1,7,-2),(-2,1,1,7,4),(0,1.04,0.96,7,-4)]
for i,(dy,sx,sy,lean,sw) in enumerate(move):
    x=95+i*118
    o.append(f'<rect x="{x-52}" y="586" width="104" height="138" rx="10" fill="#FBF9F5" stroke="#E7E1D7"/>')
    o.append(nyborg(x,704,HAIR[0][1],3,dy,sx,sy,lean,sw,ex=5))
    o.append(label(x,742,f'f{i} →',12,MUTED,'400'))
fx=880; dy,sx,sy,lean,sw=move[2]
o.append(f'<rect x="{fx-58}" y="586" width="116" height="138" rx="10" fill="#FFF7EC" stroke="#E9B872" stroke-dasharray="5 4"/>')
o.append(nyborg(fx,704,HAIR[0][1],3,dy,sx,sy,lean,sw,flip=True,ex=5))
o.append(label(fx,742,'← f2, facing left',12,INK,'600'))
o.append(label(fx,758,'(mirrored in x)',11,MUTED,'400'))
o.append('</svg>')
open(OUT,'w').write('\n'.join(o)+'\n')
