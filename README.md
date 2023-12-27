This is the software the c3spoc - the Sticker Printer Operation Center - is using to prive an API for printing stickers.

# Quick Start
Print a single image:<br>
```curl -v -F image=@<filepath> https://c3spoc.de/print/image```<br>
Print two images, rotate to landscape mode, dither image 
```curl -v -F quantity=2 -F rotate=true -F dither=true -F image=@Bilder/Sticker/fairydust.jpg https://c3spoc.de/print/image```<br>
Print a white multiline text on black background with a length of 20 mm in the Gabriella Heavy font:<br>
```curl -v -F text=$'anghenfil\n 2644' -F quantity=1 -F invert=true -F length=20 -F font="GabriellaHeavy" https://c3spoc.de/print/text```<br>

# API Specification
### GET /queue
#### Example Result
```{"jobs_todo":[],"jobs_finished_or_failed":[{"id":1,"timestamp":1703544640,"quantity":1,"status":"Complete"},{"id":2,"timestamp":1703544697,"quantity":1,"status":"Complete"},{"id":3,"timestamp":1703544777,"quantity":1,"status":"Failed"}]}```
### GET /queue/\<id\>
#### Arguments:
* id: ID of the job to get
#### Example Result
```{"id":1,"timestamp":1703544640,"quantity":1,"status":"Complete"}```
### POST /print/image
#### Arguments:
* quantity (optional): Number of stickers to print (default: 1, current max 10)
* image: jpg or png image to print on the sticker, will be resized to match the printer dimensions
* rotate (optional, bool): Rotate the image by 90° clockwise if true
* dither (optional, bool, default true): Dither the image to black and white if true, otherwise convert to b/w by thresholding
### POST /print/text
#### Arguments:
* quantity (optional): Number of stickers to print (default: 1, current max 10)
* text: Text to print on the sticker, you may use \n for newlines
* invert (optional, bool): Prints white text on black background if true. Please note that there will be a white border 
* length: maximum length of the Sticker in mm (width is fixed at 62mm)
* rotate (optional, bool, default true): Rotate the image by 90° clockwise (portrait mode = smaller stickers) if true
* font (optional, String, default Arial) Font to use for printing
    * Gabriella Heavy (37c3 Font): "GabriellaHeavy"
    * Mono Sans: "MonoSans"
    * Arial: "Arial"
    * Arial Bold: "ArialBold"
    * Arial Italic: "ArialItalic"
    * Arial Bold Italic: "ArialBoldItalic"
    * request more fonts by opening an issue
### DELETE /queue (admin only)


# Known Issues
## UTF-8 Emojis not working
Unfortunately, I couldn't get many UTF-8 Glyphs to work for the /print/text endpoint, althrough I tried merging Emoji Fonts into the font files. If you know how to fix this, please open an issue or pull request.

For now, create the image with your content yourself and send the image to the API.
