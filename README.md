# bccdc
bilibili cc字幕下载并转换格式(srt/ass/vtt)的命令行工具

# 用法
```
Usage: bccdc [option..] <bvid/mdid/epid/bcc_url/bcc_file>

Options:
    -d <directory> specify the output directory
    -c <srt/ass/vtt> specify the subtitle format to convert. default: srt
    --doc use language_name as filename instead of language_tag. (take effect while downloading with bvid/epid)
    --mixed allow pass mixed arguments
    --proxy <http://host:port> use proxy
```

# 示例
如果想下载某个视频的字幕可以直接将 `BVID` 作为参数传入

    bccdc -d downloads/ BV1kv411P7Ek

分P视频的话可以在 `BVID` 后面加上希望下载的分P或分P范围(不指定分P会下载所有分P的字幕)

    bccdc -d downloads/ BV1kv411P7Ek 1 3-4 

如果刚好在港澳台地区可以使用 `EPID` 或 `MDID` 下载这些地区的番剧的字幕

例如想下载 [ep475902](https://www.bilibili.com/bangumi/play/ep475902) 的字幕

    bccdc --proxy http://localhost:8889  ep475902

或者想下载 [md28237168](https://www.bilibili.com/bangumi/media/md28237168) 第 `3`,`4`,`10` 集的字幕(不指定集数会下载该番剧的所有字幕)

    bccdc -d downloads/ md28237168 3-4 10

或者已经找到了bcc字幕文件的url

    bccdc -d downloads/ https://i0.hdslb.com/bfs/subtitle/0f936cc0943e09cd0def198454cb00755b418fcf.json

再更进一步已经把文件下载下来了也可以

    bccdc -d downloads/ ./0f936cc0943e09cd0def198454cb00755b418fcf.json

通过上述命令执行成功后会输出已经转换好的字幕文件的路径
