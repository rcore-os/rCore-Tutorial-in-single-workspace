cd /home/chyyuu/thecodes/ai4ose
mkdir -p bundle && tar -czf bundle/tg-rcore-tutorial.tar.gz \
  --exclude='tg-rcore-tutorial/bundle' \
  --exclude='tg-rcore-tutorial/target' \
  --exclude='tg-rcore-tutorial/.git' \
  --exclude='tg-rcore-tutorial/*/target' \
  tg-rcore-tutorial
mkdir -p /home/chyyuu/thecodes/ai4ose/tg-rcore-tutorial/bundle
cp /home/chyyuu/thecodes/ai4ose/bundle/tg-rcore-tutorial.tar.gz \
   /home/chyyuu/thecodes/ai4ose/tg-rcore-tutorial/bundle/