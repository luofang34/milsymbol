from drawing_config import WORK, ROOT, SOURCES
import importlib.util,json
from pathlib import Path
import pymupdf as fitz
spec=importlib.util.spec_from_file_location('paths',str(Path(__file__).with_name('drawing_paths.py')));paths=importlib.util.module_from_spec(spec);spec.loader.exec_module(paths)
p=fitz.open(SOURCES['app6e2'])[91]
images=[e for e in paths.read(p.get_svg_image()) if 'image' in e];out={}
for name,e in zip(['restricted','no_strike'],[images[0],images[2]]):
 im=paths.Image.open(paths.io.BytesIO(paths.base64.b64decode(e['image'].split(',')[1]))).convert('RGB');a=paths.np.array(im).astype(int)
 blue=(a[:,:,2]>a[:,:,0]+40)&(a[:,:,2]>a[:,:,1]+40);black=paths.np.max(a,axis=2)<112;yellow=(a[:,:,0]>180)&(a[:,:,1]>180)&(a[:,:,2]<128)
 yy,xx=paths.np.nonzero(blue);scale=100/(xx.max()-xx.min());yy,xx=paths.np.nonzero(black|yellow);cx=(xx.min()+xx.max())/2;cy=(yy.min()+yy.max())/2
 m=paths.Transform(scale,0,0,scale,-cx*scale,-cy*scale)
 result=[]
 for color,mask in [('#ffff00',yellow),('#000000',black)]:
  d=paths.image_paths(paths.Image.fromarray(paths.np.where(mask,0,255).astype('uint8')))
  result.append({'d':paths.transform_path(d,m),'fill':color,'stroke':None,'width':0})
 out[name]={'paths':result,'source':{'document':'app6e2','printed':58,'figure':'1-6' if name=='restricted' else '1-7'},'bbox':[(xx.min()-cx)*scale,(yy.min()-cy)*scale,(xx.max()-cx)*scale,(yy.max()-cy)*scale]}
Path(str(WORK / 'context-icons.json')).write_text(json.dumps(out));print([(n,x['bbox']) for n,x in out.items()])
