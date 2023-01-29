# config_editor

## 目的
excel在编辑数据的时候很好用, 可以通过公式, 拖拽等方式来快速生成大量数据  
不过 excel 在多人协作的时候会相互锁, 降低工作效率
打算通过一个工具来解决这个问题

## 设计思路
将excel的数据转换成 json 格式, 然后通过不同的方式将json数据转换成 csv, excel, json 等
并且支持 excel 文件的导入导出, 方便利用excel来批量编辑
