from importlib import reload
import load
reload(load)
from load import *
input = input.reshape(1080, 1920)
black_level = black_level.reshape(1080, 1920)
output = output.reshape(1080, 1920, 4)[:, :, :3].clip(0, 1023)
cfa = np.fromfile("cfa.bin", dtype = "uint16").reshape(1080, 1920, 3)
print(output.max())

# output /= output.max()

plt.imshow(output / output.max())
plt.show()