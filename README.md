# hif_2_jpeg_with_gainmaps
A rust program that maps a sony hif file to a jpg with gain maps
This program is still in development. The intention is to add a gui in slint in which
an HIF file can be selected and the intended output jpg file.  
There will be two parameters: gain (usually <1) for the SDR and gain (usually 1 or larger) for the HDR
The SDR is created via Yu'v' and thereby automatically preserving colors. All colors lost in the 
conversion to BT.709 will be regained when the gain map is applied because the picture will be 
regenerated in the BT.2020 domain given a BT.2020 image of the SDR with lost colors. The 
gain map process will successfully recover those lost colors (formally spoken in the Hue-Saturation language
BT.2020 does not have more hues than BT.709, it only has more saturation for some hue values than BT.709 does.
So the gain map process restores the larger saturation values supported by BT.2020. 

