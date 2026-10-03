# SPDX-FileCopyrightText: 2026 Nye Warburton
# SPDX-License-Identifier: MIT OR Apache-2.0
# Writes docs/design/nyborg-mockup.svg (revision 2: felt Muppet-style hair, primary colours).
# Layers (defs): head, face (eyes + mouth), hair (fill inherited = runtime tint), glasses, hat.
OUT='/workspace/blitzwing/docs/design/nyborg-mockup.svg'
HEAD='#EADFCB'; LINE='#5E544B'; EYE='#2F2A26'; BG='#F5F2EC'; INK='#5E544B'; MUTED='#8C8378'
HAIR=[('Felt Red','#D9534F','default'),('Cobalt','#3F6FD8','blue'),('Sunny','#F2C14E','yellow'),('Magenta','#C8459A','extra')]
# hair locks: (cx, cy, r) relative to the crown root (0,0) = top of the head
import math
def _r(i):  # deterministic jitter in [0, 1)
    return ((i*2654435761) % 1000)/1000.0
def hair_path():
    # A shaggy mop around (0,-9): many soft, slightly pointed locks with jittered length.
    # Angles are SVG angles (0 = +x, 90 = down). The bottom tucks into the head; one lock flops left.
    pts=[]; n=26
    for i in range(n):
        a=math.radians(i*360/n)
        down=max(0.0,math.sin(a))             # 1 at the bottom
        base=19-7*down                         # shorter at the bottom, so it sits on the crown
        r=base+5.5*_r(i+3)*(1-0.6*down)
        ad=math.degrees(a)%360
        if 150<ad<200: r+=6*(1-abs(ad-175)/25)  # floppy lock over the left side
        pts.append((r*math.cos(a)*1.08, -10+r*math.sin(a)*0.98))
    d=f"M{(pts[0][0]+pts[1][0])/2:.1f},{(pts[0][1]+pts[1][1])/2:.1f}"
    for i in range(n):
        p0=pts[(i+1)%n]; p1=pts[(i+2)%n]
        # control point pushed outward a bit -> rounded, ragged lobes
        cx,cy=p0[0]*1.1,(p0[1]+10)*1.1-10
        d+=f" Q{cx:.1f},{cy:.1f} {(p0[0]+p1[0])/2:.1f},{(p0[1]+p1[1])/2:.1f}"
    return d+" Z"
def hair_def():
    d=hair_path()
    strands=''
    # short felt curls scattered inside the mop: light ones on top, dark ones low
    for k in range(16):
        x=-15+30*_r(k*7+1); y=-26+26*_r(k*11+5)
        if (x/17)**2+((y+10)/16)**2>0.8: continue
        w=3+2.5*_r(k+9); flip=1 if k%2 else -1
        col,op=('#fff','0.35') if y<-12 else ('#000','0.16')
        strands+=f'<path d="M{x-w:.1f},{y:.1f} q{w:.1f},{-2.6*flip:.1f} {2*w:.1f},0" stroke="{col}" stroke-opacity="{op}"/>'
    return (f'<g id="hair"><g filter="url(#fuzz)"><path d="{d}" stroke="{LINE}" stroke-width="2.2" stroke-linejoin="round"/></g>'
            f'<g filter="url(#fuzz2)" fill="none" stroke-width="1.5" stroke-linecap="round">{strands}</g></g>')
def nyborg(x,y,hair,dy=0,sx=1,sy=1,lean=0,tw=0,hy=0,acc=None,flip=False,s=1.0,ex=0,mouth='smile'):
    sh=max(0.55,1-abs(dy)/14)
    g=f'<g transform="translate({x},{y}) scale({s})">'
    g+=f'<ellipse cx="0" cy="0" rx="{22*sh:.1f}" ry="{4.2*sh:.1f}" fill="#000" opacity="0.10"/>'
    inner=f'<g transform="translate(0,{dy}) rotate({lean}) scale({sx},{sy})">'
    inner+='<use href="#head"/>'
    inner+=f'<g transform="translate(0,{-58+hy}) rotate({tw})"><use href="#hair" fill="{hair}"/></g>'
    inner+=f'<g transform="translate({ex},0)"><use href="#eyes"/><use href="#mouth-{mouth}"/>'
    if acc=='glasses': inner+='<use href="#glasses"/>'
    inner+='</g>'
    if acc=='hat': inner+=f'<g transform="translate(0,{-73+hy}) rotate({tw*0.5})"><use href="#hat"/></g>'
    inner+='</g>'
    if flip: inner=f'<g transform="scale(-1,1)">{inner}</g>'
    return g+inner+'</g>'
def label(x,y,t,size=13,color=INK,weight='600',anchor='middle'):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" font-weight="{weight}" text-anchor="{anchor}">{t}</text>'
W,H=1000,780
o=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="DejaVu Sans, Helvetica, Arial, sans-serif">',
'<title>Nyborg character mockup, revision 2 (design only)</title>',
'<defs>',
# fuzz: fractal noise displaces the lock edges so they read as felt/fur
'<filter id="fuzz" x="-30%" y="-30%" width="160%" height="160%"><feTurbulence type="fractalNoise" baseFrequency="1.0" numOctaves="2" seed="7" result="n"/>'
'<feDisplacementMap in="SourceGraphic" in2="n" scale="4.2" xChannelSelector="R" yChannelSelector="G"/></filter>',
'<filter id="fuzz2" x="-30%" y="-30%" width="160%" height="160%"><feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="1" seed="3" result="n"/>'
'<feDisplacementMap in="SourceGraphic" in2="n" scale="2.5" xChannelSelector="R" yChannelSelector="G"/></filter>',
f'<g id="head"><circle cx="0" cy="-31" r="31" fill="{HEAD}" stroke="{LINE}" stroke-width="2.6"/>'
f'<ellipse cx="-12" cy="-40" rx="8" ry="4.5" fill="#fff" opacity="0.30" transform="rotate(-25 -12 -40)"/></g>',
f'<g id="eyes"><circle cx="-10" cy="-30" r="4" fill="{EYE}"/><circle cx="10" cy="-30" r="4" fill="{EYE}"/>'
'<circle cx="-8.8" cy="-31.4" r="1.2" fill="#fff"/><circle cx="11.2" cy="-31.4" r="1.2" fill="#fff"/></g>',
f'<path id="mouth-smile" d="M-11,-19 Q0,-10 11,-19" fill="none" stroke="{EYE}" stroke-width="2.4" stroke-linecap="round"/>',
f'<g id="mouth-open"><path d="M-12,-20 Q0,-5 12,-20 Q0,-17 -12,-20 Z" fill="#4A2F2C" stroke="{EYE}" stroke-width="2" stroke-linejoin="round"/>'
'<path d="M-5,-12.6 Q0,-9.6 5,-12.6 Q0,-14.6 -5,-12.6 Z" fill="#E98A86"/></g>',
hair_def(),
f'<g id="glasses" fill="none" stroke="#3B3632" stroke-width="2.2"><circle cx="-10" cy="-30" r="7.8" fill="#fff" fill-opacity="0.25"/>'
'<circle cx="10" cy="-30" r="7.8" fill="#fff" fill-opacity="0.25"/><path d="M-2.2,-31 Q0,-33 2.2,-31"/><path d="M-17.8,-32 L-28,-35 M17.8,-32 L28,-35"/></g>',
# hat: sits on top of the hair (root = crown), brim over the fuzz, tilted a little
f'<g id="hat" stroke="{LINE}" stroke-width="2.2" stroke-linejoin="round" transform="rotate(10 0 -10)">'
'<ellipse cx="2" cy="-9" rx="24" ry="5" fill="#5F7A99"/>'
'<path d="M-12,-10 C-12,-24 -6,-30 2,-30 C10,-30 16,-24 16,-10 Z" fill="#6E8BAE"/>'
'<rect x="-11.4" y="-16" width="26.8" height="4.6" fill="#F5F2EC" stroke="none"/></g>',
'</defs>',
f'<rect width="{W}" height="{H}" fill="{BG}"/>',
f'<rect x="12" y="12" width="{W-24}" height="{H-24}" rx="18" fill="none" stroke="#E2DCD2" stroke-width="2"/>',
label(40,52,'Nyborg — character mockup (revision 2)',22,INK,'700','start'),
label(40,74,'Design only (gate draft). Felt, Muppet-style hair. Layers: head, face, hair (tinted at runtime), accessory.',13,MUTED,'400','start'),
]
o.append(label(40,112,'Hair colours',15,INK,'700','start'))
o.append(label(690,112,'Upgrades (cosmetic only)',15,INK,'700','start'))
xs=[95,235,375,515]
for (n,c,k),x in zip(HAIR,xs):
    o.append(nyborg(x,256,c,s=1.35))
    o.append(label(x,284,n)); o.append(label(x,300,f'{c} · {k}',11,MUTED,'400'))
o.append(nyborg(735,256,HAIR[1][1],acc='glasses',s=1.35)); o.append(label(735,284,'Glasses')); o.append(label(735,300,'Cobalt hair',11,MUTED,'400'))
o.append(nyborg(880,256,HAIR[2][1],acc='hat',s=1.35)); o.append(label(880,284,'Hat')); o.append(label(880,300,'Sunny hair',11,MUTED,'400'))
o.append(f'<line x1="40" x2="{W-40}" y1="318" y2="318" stroke="#E2DCD2" stroke-width="2"/>')
o.append(label(40,352,'Idle loop — 4 frames at 6 fps (gentle breathing bob)',15,INK,'700','start'))
idle=[(0,1.04,0.96,0,1),(-1,1,1,3,0),(-2,0.98,1.03,0,-1),(-1,1,1,-3,0)]
for i,(dy,sx,sy,tw,hy) in enumerate(idle):
    x=110+i*130
    o.append(f'<rect x="{x-55}" y="366" width="110" height="136" rx="10" fill="#FBF9F5" stroke="#E7E1D7"/>')
    o.append(nyborg(x,484,HAIR[0][1],dy,sx,sy,0,tw,hy))
    o.append(label(x,520,f'f{i}',12,MUTED,'400'))
o.append(label(640,410,'Cell 112×112 px, pivot at the feet (56, 104).',12,MUTED,'400','start'))
o.append(label(640,430,'Squash on f0, rise on f2;',12,MUTED,'400','start'))
o.append(label(640,450,'the hair settles a beat behind the head.',12,MUTED,'400','start'))
o.append(f'<line x1="40" x2="{W-40}" y1="540" y2="540" stroke="#E2DCD2" stroke-width="2"/>')
o.append(label(40,572,'Move loop — 6 frames at 10 fps (bounce, lean, floppy hair, mouth opens)',15,INK,'700','start'))
# (dy, sx, sy, lean, hair tilt, hair lag y, mouth)
move=[(0,1.06,0.94,7,-8,3,'smile'),(-4,1,1,7,-14,4,'open'),(-8,0.97,1.04,6,-6,1,'open'),(-6,1,1,7,7,-2,'open'),(-2,1,1,7,12,-1,'smile'),(0,1.04,0.96,7,2,2,'smile')]
for i,(dy,sx,sy,lean,tw,hy,m) in enumerate(move):
    x=95+i*118
    o.append(f'<rect x="{x-52}" y="586" width="104" height="138" rx="10" fill="#FBF9F5" stroke="#E7E1D7"/>')
    o.append(nyborg(x,704,HAIR[0][1],dy,sx,sy,lean,tw,hy,ex=5,mouth=m))
    o.append(label(x,742,f'f{i} →',12,MUTED,'400'))
fx=880; dy,sx,sy,lean,tw,hy,m=move[2]
o.append(f'<rect x="{fx-58}" y="586" width="116" height="138" rx="10" fill="#FFF7EC" stroke="#E9B872" stroke-dasharray="5 4"/>')
o.append(nyborg(fx,704,HAIR[0][1],dy,sx,sy,lean,tw,hy,flip=True,ex=5,mouth=m))
o.append(label(fx,742,'← f2, facing left',12,INK,'600'))
o.append(label(fx,758,'(mirrored in x)',11,MUTED,'400'))
o.append('</svg>')
open(OUT,'w').write('\n'.join(o)+'\n')
