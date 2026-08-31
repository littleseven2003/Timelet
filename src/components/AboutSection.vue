<script setup lang="ts">
// 关于时屿（设计文档 5.9）：定位、理念与本地隐私说明；版本与开源地址在发布准备时补全
import appIcon from '../assets/app-icon.png';
</script>

<template>
  <section class="about">
    <div class="about__hero">
      <img class="about__icon" :src="appIcon" alt="" />
      <div>
        <div class="about__name">时屿</div>
        <div class="about__wordmark">TIMELET · 倒数日 · 正数日 · 提醒</div>
      </div>
    </div>

    <p class="about__slogan">把重要的日子，安静地放在菜单栏。</p>

    <div class="about__block">
      <h3 class="about__heading">设计理念</h3>
      <p>每一个重要时刻，都值得被记住。时弧表示仍在进行的时间，小岛是承载它的地方。</p>
    </div>

    <div class="about__block">
      <h3 class="about__heading">本地与隐私</h3>
      <p>条目与设置仅保存在本机，无需登录账号。</p>
    </div>

    <div class="about__block">
      <h3 class="about__heading">版本</h3>
      <p>v0.1.0 · GPL-3.0</p>
    </div>
  </section>
</template>

<style scoped>
.about {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 520px;
}

.about__hero {
  display: flex;
  align-items: center;
  gap: 14px;
}

.about__icon {
  width: 56px;
  height: 56px;
}

.about__name {
  font-size: 21px;
  font-weight: 500;
  letter-spacing: 0.08em;
}

.about__wordmark {
  font-size: 11px;
  letter-spacing: 0.14em;
  color: var(--ts-text-2);
  margin-top: 3px;
}

.about__slogan {
  font-size: 14px;
  margin: 0;
}

.about__block {
  border-top: 1px solid var(--ts-line);
  padding-top: 12px;
}

.about__heading {
  font-size: 12px;
  font-weight: 600;
  color: var(--ts-text-2);
  margin: 0 0 6px;
}

.about__block p {
  font-size: 13px;
  margin: 0;
  line-height: 1.6;
}
</style>
