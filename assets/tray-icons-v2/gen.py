import math, io, os, shutil, struct, cairosvg, numpy as np
from PIL import Image
W=256
CLOUD='<circle cx="70" cy="146" r="30"/><circle cx="116" cy="112" r="44"/><circle cx="168" cy="124" r="36"/><circle cx="194" cy="148" r="28"/><rect x="70" y="130" width="124" height="46"/>'
CX,CY,RX,RY,ROT=128,132,118,34,-20; RW=7; GAP=5
def pt(t):
    x,y=RX*math.cos(math.radians(t)),RY*math.sin(math.radians(t)); r=math.radians(ROT)
    return CX+x*math.cos(r)-y*math.sin(r), CY+x*math.sin(r)+y*math.cos(r)
SX,SY=pt(200); SR=11
ELL=lambda w,c:f'<ellipse cx="{CX}" cy="{CY}" rx="{RX}" ry="{RY}" transform="rotate({ROT} {CX} {CY})" fill="none" stroke="{c}" stroke-width="{w}"/>'
FRONT=lambda w,c:f'<path d="M{CX+RX*math.cos(math.radians(55)):.1f} {CY+RY*math.sin(math.radians(55)):.1f} A{RX} {RY} 0 0 1 {CX-RX} {CY}" transform="rotate({ROT} {CX} {CY})" fill="none" stroke="{c}" stroke-width="{w}" stroke-linecap="round"/>'
BX,BY,BR=200,200,51; KN=BR+4
COL={'synced':'#34C759','syncing':'#0A84FF','disconnected':'#FF3B30'}
def glyph(k):
    s='stroke="#fff" stroke-width="12" fill="none" stroke-linecap="round" stroke-linejoin="round"'
    if k=='synced': return f'<path d="M{BX-22} {BY+1} L{BX-7} {BY+17} L{BX+23} {BY-15}" {s}/>'
    if k=='disconnected': return f'<path d="M{BX-17} {BY-17} L{BX+17} {BY+17} M{BX+17} {BY-17} L{BX-17} {BY+17}" {s}/>'
    R=24
    p=lambda a:(BX+R*math.cos(math.radians(a)),BY+R*math.sin(math.radians(a)))
    def arc(a0,a1):(x0,y0),(x1,y1)=p(a0),p(a1);return f'M{x0:.1f} {y0:.1f} A{R} {R} 0 0 1 {x1:.1f} {y1:.1f}'
    def head(a):
        (ax,ay)=p(a);t=math.radians(a+90);n=math.radians(a)
        tip=(ax+12*math.cos(t),ay+12*math.sin(t));b1=(ax+11*math.cos(n),ay+11*math.sin(n));b2=(ax-11*math.cos(n),ay-11*math.sin(n))
        return f'<path d="M{b1[0]:.1f} {b1[1]:.1f} L{tip[0]:.1f} {tip[1]:.1f} L{b2[0]:.1f} {b2[1]:.1f}Z" fill="#fff" stroke="#fff" stroke-width="3" stroke-linejoin="round"/>'
    return f'<path d="{arc(200,318)} {arc(20,138)}" stroke="#fff" stroke-width="10" fill="none" stroke-linecap="round"/>'+head(318)+head(138)
def badge(k,variant):
    out=f' stroke="#1C1C1E" stroke-width="2"' if variant=='white' else ''
    return f'<circle cx="{BX}" cy="{BY}" r="{BR}" fill="{COL[k]}"{out}/>'+glyph(k)
def svg(k,variant):
    c='#FFFFFF' if variant=='white' else '#1C1C1E'; b=k!='base'
    kn=f'<circle cx="{BX}" cy="{BY}" r="{KN}" fill="black"/>' if b else ''
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{W}" viewBox="0 0 {W} {W}"><defs>
<mask id="cm" maskUnits="userSpaceOnUse" x="0" y="0" width="{W}" height="{W}"><rect width="{W}" height="{W}" fill="white"/>{FRONT(RW+2*GAP,"black")}{kn}</mask>
<mask id="rm" maskUnits="userSpaceOnUse" x="0" y="0" width="{W}" height="{W}"><rect width="{W}" height="{W}" fill="white"/><g fill="black">{CLOUD}</g>{FRONT(RW+1,"white")}{kn}</mask></defs>
<g fill="{c}" mask="url(#cm)">{CLOUD}</g><g mask="url(#rm)">{ELL(RW,c)}<circle cx="{SX:.1f}" cy="{SY:.1f}" r="{SR}" fill="{c}"/></g>{badge(k,variant) if b else ''}</svg>'''
# PNG compositing (cairosvg ignores <mask>)
def L(inner,ss=4):
    s=f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{W}" viewBox="0 0 {W} {W}">{inner}</svg>'
    return np.asarray(Image.open(io.BytesIO(cairosvg.svg2png(bytestring=s.encode(),output_width=W*ss,output_height=W*ss))).convert('RGBA')).astype(float)/255
a=lambda i:L(i)[...,3]
C=a(f'<g fill="#fff">{CLOUD}</g>');FG=a(FRONT(RW+2*GAP,'#fff'));F=a(FRONT(RW,'#fff'));R=a(ELL(RW,'#fff'))
S=a(f'<circle cx="{SX:.1f}" cy="{SY:.1f}" r="{SR}" fill="#fff"/>');K=a(f'<circle cx="{BX}" cy="{BY}" r="{KN}" fill="#fff"/>')
BASEA=np.maximum.reduce([C*(1-FG),(R*(1-C)),F,S*(1-C)])
def png(k,variant):
    rgb=(1,1,1) if variant=='white' else (0x1C/255,0x1C/255,0x1E/255)
    out=np.zeros(BASEA.shape+(4,));out[...,:3]=rgb;A=BASEA.copy()
    if k!='base': A*=1-K
    out[...,3]=A
    if k!='base':
        B=L(badge(k,variant));ba=B[...,3:4];oa=out[...,3:4];na=ba+oa*(1-ba)
        out[...,:3]=(B[...,:3]*ba+out[...,:3]*oa*(1-ba))/np.maximum(na,1e-6);out[...,3:4]=na
    return Image.fromarray((out*255).round().astype(np.uint8),'RGBA').resize((W,W),Image.LANCZOS)
names=['base','synced','syncing','disconnected']
SIZES=(64,48,32,24,22,20)  # downscaled from the 256 master
for d in ['256',*map(str,SIZES),'16','16x8','ico']: os.makedirs(d,exist_ok=True)
for v in ['white','dark']:
    for k in names:
        n=f'{v}-{k}'; open(f'256/{n}.svg','w').write(svg(k,v)); im=png(k,v); im.save(f'256/{n}.png')
        for s in SIZES: im.resize((s,s),Image.LANCZOS).save(f'{s}/{n}.png')
# 16px hand pixel art
G=["................","................","................","................",
   "......####......",".....######.....","..##.######.##..",".##############.",
   "################","################","################",".##############.",
   "................","................","................","................"]
cloud=np.array([[ch=='#' for ch in r] for r in G])
DOT=["..###..",".#####.","#######","#######","#######",".#####.","..###.."]
dot=np.zeros((16,16),bool); 
for i,r in enumerate(DOT):
    for j,ch in enumerate(r): dot[9+i,9+j]= ch=='#'
dil=dot.copy()
for dy in(-1,0,1):
    for dx in(-1,0,1): dil|=np.roll(np.roll(dot,dy,0),dx,1)
for v in ['white','dark']:
    rgb=(255,255,255) if v=='white' else (0x1C,0x1C,0x1E)
    for k in names:
        arr=np.zeros((16,16,4),np.uint8); m=cloud&~dil if k!='base' else cloud
        arr[m]=rgb+(255,)
        if k!='base': h=COL[k];arr[dot]=(int(h[1:3],16),int(h[3:5],16),int(h[5:7],16),255)
        im=Image.fromarray(arr,'RGBA');im.save(f'16/{v}-{k}.png');im.resize((128,128),Image.NEAREST).save(f'16x8/{v}-{k}.png')
# preview
cols=8; cw=280
sheet=Image.new('RGB',(cols*cw+20,2*470))
for row,bg in enumerate([(0x1E,0x1E,0x1E),(0xEC,0xEC,0xEC)]):
    band=Image.new('RGBA',(sheet.width,470),bg+(255,))
    for i,v in enumerate(['white','dark']):
        for j,k in enumerate(names):
            n=f'{v}-{k}';x=10+(i*4+j)*cw
            band.alpha_composite(Image.open(f'256/{n}.png'),(x,10))
            band.alpha_composite(Image.open(f'64/{n}.png'),(x,290));band.alpha_composite(Image.open(f'32/{n}.png'),(x+75,306))
            band.alpha_composite(Image.open(f'16/{n}.png'),(x+120,314));band.alpha_composite(Image.open(f'16x8/{n}.png'),(x+145,290))
    sheet.paste(band.convert('RGB'),(0,row*470))
sheet.save('preview.png')
for root,_,fs in os.walk('.'):
    for f in fs:
        if f.endswith('.png') and f!='preview.png':
            im=Image.open(os.path.join(root,f));assert im.mode=='RGBA' and im.getpixel((0,0))[3]==0
# Multi-size .ico for Windows, one file per variant and state.
# 16 px uses the hand-made pixel art; every other size is downscaled from the 256 master.
ICO_SIZES=(16,20,24,32,48,64,256)
def ico_frames(k,v):
    src={16:f'16/{v}-{k}.png',256:f'256/{v}-{k}.png'}
    return {s:Image.open(src.get(s,f'{s}/{v}-{k}.png')).convert('RGBA') for s in ICO_SIZES}
def write_ico(path,frames):
    blobs=[]
    for s,im in frames.items():
        b=io.BytesIO(); im.save(b,'PNG'); blobs.append((s,b.getvalue()))
    out=bytearray(struct.pack('<HHH',0,1,len(blobs)))
    offset=6+16*len(blobs)
    for s,data in blobs:
        w=0 if s==256 else s  # ICO stores 256 as 0
        out+=struct.pack('<BBBBHHII',w,w,0,0,1,32,len(data),offset); offset+=len(data)
    for _,data in blobs: out+=data
    open(path,'wb').write(bytes(out))
for v in ['white','dark']:
    for k in names: write_ico(f'ico/{v}-{k}.ico', ico_frames(k,v))
print('ok')
