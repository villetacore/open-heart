import json
from pathlib import Path
from PIL import Image
root=Path("game/assets")
manifest=[]
def atlas(folder, file, names, cols, rows):
 path=root/folder/file
 im=Image.open(path); w,h=im.size
 for i,name in enumerate(names):
  # Safe inset avoids generated grid lines and neighboring cells.
  x=round((i%cols)*w/cols)+5; y=round((i//cols)*h/rows)+5
  cw=round(((i%cols)+1)*w/cols)-x-5; ch=round(((i//cols)+1)*h/rows)-y-5
  out=root/folder/(name+".tres")
  out.write_text('[gd_resource type="AtlasTexture" load_steps=2 format=3]\n\n[ext_resource type="Texture2D" path="res://assets/'+folder+'/'+file+'" id="1"]\n\n[resource]\natlas = ExtResource("1")\nregion = Rect2('+f'{x}, {y}, {cw}, {ch}'+')\n',encoding="utf-8")
  manifest.append(dict(id=name,path=str(out).replace("\\","/"),atlas=str(path).replace("\\","/"),region=[x,y,cw,ch]))
people=["ivo","noel","lucien","ash","emil","yves"]
atlas("portraits/neighbors","modern_atlas.png",people,3,2)
atlas("icons/neighbors","atlas.png",people+[p+"_2" for p in people],4,3)
atlas("decor/neighbors","panels.png",["rose","archive","music","repair","bread","garden","dance","letter","mask","heart","moth","compass"],4,3)
# Cosmetics .tres live in the existing lookup directory.
styles=["moon_brooch","jasmine_pin","silk_cravat","lace_fan","heart_ring","voltage_bow","opera_mask","rose_beret","heart_watch","moon_collar","pearl_cuff","rose_glove"]
tech=["led_choker","platform_boots","cyber_bow","trans_pin","punk_cuff","ar_visor","mesh_sleeves","tech_skirt","heart_headphones","neon_liner","cyber_nails","punk_bag"]
for filename,names in [("cosmetics_neighborhood_atlas.png",styles),("cosmetics_tech_atlas.png",tech)]:
 if not (root/"icons"/filename).exists(): continue
 atlas("icons",filename,names,4,3)
 for name in names:
  src=root/"icons"/(name+".tres"); dst=root/"icons/cosmetics"/(name+".tres")
  dst.write_text(src.read_text(encoding="utf-8"),encoding="utf-8"); src.unlink()
  manifest[-len(names)+names.index(name)]["path"]=str(dst).replace("\\","/")
if (root/"illustrations/neighbors/modern_places.png").exists():
 atlas("illustrations/neighbors","modern_places.png",people,3,2)
(root/"portraits/neighbors/manifest.json").write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print("Atlas regions:",len(manifest))

