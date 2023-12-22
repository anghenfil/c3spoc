This is the software the c3spoc - the Sticker Printer Operation Center - is using to prive an API for printing stickers.

# Quick Start
Print a single image:<br>
```curl -v -F image=@<filepath> http://<ip>/print/image```<br>
Print a single image with a length of 100mm:<br>
```curl -v -F image=@<filepath> -F length=100 http://<ip>/print/image```<br>

# API Specification
### GET /queue?status=\<status\>
#### Arguments:
* status (optional): "pending", "printing", "printed", "failed", "all" (default: pending)<br>
Returns a list of all jobs in the queue. The list is sorted by the time the jobs were added to the queue.
```json
[ 
  {
    "id": 1,
    "timestamp": "2018-12-27T15:00:00Z",
    "status": "pending",
    "quantity": 1
  },
  {
    "id": 2,
    "timestamp": "2018-12-27T15:00:00Z",
    "status": "printing",
    "quantity": 10
  }
]
```
### GET /queue/\<id\>
### POST /print/image
#### Arguments:
* quantity (optional): Number of stickers to print (default: 1, current max 10)
* image: jpg or png image to print on the sticker
* length (optional): Length of the Sticker in mm (width is fixed at 62mm), if not set the image will be scaled to fit the width
* rotate (optional, true or false): Rotate the image by 90° clockwise
### POST /print/text
#### Arguments:
* quantity (optional): Number of stickers to print (default: 1, current max 10)
* text: Text to print on the sticker
* length: Length of the Sticker in mm (width is fixed at 62mm)
### DELETE /queue (admin only)
