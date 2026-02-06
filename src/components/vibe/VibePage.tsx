/**
 * Vibe 页面组件
 *
 * 一个充满活力和创意的页面，展示应用的状态和氛围
 * 包含动态视觉效果、状态指示器和创意元素
 */

import React, { useState, useEffect } from "react";
import styled from "styled-components";
import {
  Music,
  Sparkles,
  Zap,
  Activity,
  Palette,
  Brain,
  Heart,
  TrendingUp,
  Users,
  Globe,
  Cpu,
  Battery,
  Wifi,
  Radio,
  Volume2,
  Sun,
  Droplets,
} from "lucide-react";
import { Button } from "@/components/ui/button";

const VibeContainer = styled.div`
  flex: 1;
  padding: 24px;
  overflow-y: auto;
  background: linear-gradient(
    135deg,
    hsl(var(--background)) 0%,
    hsl(var(--card)) 50%,
    hsl(var(--muted)) 100%
  );
  min-height: 100%;
`;

const Header = styled.div`
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 32px;
`;

const TitleSection = styled.div`
  display: flex;
  align-items: center;
  gap: 16px;
`;

const Title = styled.h1`
  font-size: 2.5rem;
  font-weight: 800;
  background: linear-gradient(
    135deg,
    hsl(var(--primary)) 0%,
    hsl(var(--secondary)) 50%,
    hsl(var(--accent)) 100%
  );
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  margin: 0;
`;

const Subtitle = styled.p`
  font-size: 1.1rem;
  color: hsl(var(--muted-foreground));
  margin-top: 8px;
  max-width: 600px;
`;

const StatsGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 20px;
  margin-bottom: 32px;
`;

const StatCard = styled.div`
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: 16px;
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  transition: all 0.3s ease;
  cursor: pointer;

  &:hover {
    transform: translateY(-4px);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
    border-color: hsl(var(--primary));
  }
`;

const StatIcon = styled.div<{ $color: string }>`
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: ${(props) => props.$color};
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
`;

const StatContent = styled.div`
  flex: 1;
`;

const StatValue = styled.div`
  font-size: 1.8rem;
  font-weight: 700;
  color: hsl(var(--foreground));
`;

const StatLabel = styled.div`
  font-size: 0.9rem;
  color: hsl(var(--muted-foreground));
  margin-top: 4px;
`;

const VisualGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
  margin-bottom: 32px;
`;

const VisualCard = styled.div`
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: 20px;
  padding: 24px;
  overflow: hidden;
  position: relative;
`;

const VisualHeader = styled.div`
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
`;

const VisualTitle = styled.h3`
  font-size: 1.3rem;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin: 0;
`;

const WaveContainer = styled.div`
  height: 120px;
  position: relative;
  overflow: hidden;
  border-radius: 12px;
  background: linear-gradient(
    135deg,
    hsl(var(--primary) / 0.1) 0%,
    hsl(var(--secondary) / 0.1) 100%
  );
`;

const Wave = styled.div<{ $delay: number }>`
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 40px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    hsl(var(--primary) / 0.6) 50%,
    transparent 100%
  );
  animation: wave ${(props) => 2 + props.$delay}s ease-in-out infinite;
  animation-delay: ${(props) => props.$delay}s;

  @keyframes wave {
    0%,
    100% {
      transform: translateX(-100%);
    }
    50% {
      transform: translateX(100%);
    }
  }
`;

const ParticleGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 8px;
  height: 120px;
  align-items: end;
`;

const Particle = styled.div<{ $height: number; $delay: number }>`
  background: linear-gradient(
    to top,
    hsl(var(--primary)) 0%,
    hsl(var(--secondary)) 100%
  );
  height: ${(props) => props.$height}%;
  border-radius: 4px 4px 0 0;
  animation: pulse ${(props) => 1 + props.$delay}s ease-in-out infinite;
  animation-delay: ${(props) => props.$delay}s;

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.6;
    }
    50% {
      opacity: 1;
    }
  }
`;

const ColorGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 12px;
`;

const ColorSwatch = styled.div<{ $color: string }>`
  aspect-ratio: 1;
  border-radius: 8px;
  background: ${(props) => props.$color};
  cursor: pointer;
  transition: transform 0.2s ease;

  &:hover {
    transform: scale(1.1);
  }
`;

const MoodSection = styled.div`
  display: flex;
  flex-direction: column;
  gap: 20px;
`;

const MoodSelector = styled.div`
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
`;

const MoodButton = styled(Button)<{ $active: boolean }>`
  flex: 1;
  min-width: 120px;
  background: ${(props) =>
    props.$active ? "hsl(var(--primary))" : "hsl(var(--secondary))"};
  color: ${(props) =>
    props.$active
      ? "hsl(var(--primary-foreground))"
      : "hsl(var(--secondary-foreground))"};
  border: 2px solid
    ${(props) => (props.$active ? "hsl(var(--primary))" : "transparent")};

  &:hover {
    background: ${(props) =>
      props.$active ? "hsl(var(--primary))" : "hsl(var(--secondary))"};
    opacity: 0.9;
  }
`;

const MoodDisplay = styled.div<{ $mood: string }>`
  height: 80px;
  border-radius: 16px;
  background: ${(props) => {
    switch (props.$mood) {
      case "energetic":
        return "linear-gradient(135deg, #FF6B6B, #FFE66D)";
      case "calm":
        return "linear-gradient(135deg, #4ECDC4, #556270)";
      case "creative":
        return "linear-gradient(135deg, #9D50BB, #6E48AA)";
      case "focused":
        return "linear-gradient(135deg, #00B4DB, #0083B0)";
      default:
        return "linear-gradient(135deg, hsl(var(--primary)), hsl(var(--secondary)))";
    }
  }};
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 1.2rem;
  font-weight: 600;
  transition: all 0.5s ease;
`;

const ActionButtons = styled.div`
  display: flex;
  gap: 16px;
  margin-top: 32px;
  justify-content: center;
`;

const TimeDisplay = styled.div`
  font-size: 3rem;
  font-weight: 300;
  font-family: "SF Mono", monospace;
  color: hsl(var(--foreground));
  text-align: center;
  margin: 20px 0;
`;

const DateDisplay = styled.div`
  font-size: 1.1rem;
  color: hsl(var(--muted-foreground));
  text-align: center;
  margin-bottom: 32px;
`;

export function VibePage() {
  const [currentTime, setCurrentTime] = useState(new Date());
  const [selectedMood, setSelectedMood] = useState("creative");
  const [activeVisual, setActiveVisual] = useState("waves");

  // 更新时间
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // 生成随机粒子高度
  const particles = Array.from({ length: 8 }, () => ({
    height: Math.random() * 60 + 40,
    delay: Math.random() * 2,
  }));

  // 颜色样本
  const colors = [
    "hsl(var(--primary))",
    "hsl(var(--secondary))",
    "hsl(var(--accent))",
    "hsl(var(--destructive))",
    "hsl(var(--success))",
    "#FF6B6B",
    "#4ECDC4",
    "#9D50BB",
    "#FFE66D",
    "#556270",
  ];

  // 统计数据
  const stats = [
    {
      icon: <Brain size={24} />,
      value: "98%",
      label: "AI 活跃度",
      color: "linear-gradient(135deg, #9D50BB, #6E48AA)",
    },
    {
      icon: <Zap size={24} />,
      value: "256",
      label: "今日请求数",
      color: "linear-gradient(135deg, #FF6B6B, #FFE66D)",
    },
    {
      icon: <Users size={24} />,
      value: "12",
      label: "在线用户",
      color: "linear-gradient(135deg, #4ECDC4, #556270)",
    },
    {
      icon: <Globe size={24} />,
      value: "8",
      label: "活跃模型",
      color: "linear-gradient(135deg, #00B4DB, #0083B0)",
    },
    {
      icon: <Cpu size={24} />,
      value: "42ms",
      label: "平均延迟",
      color: "linear-gradient(135deg, #FF9A9E, #FAD0C4)",
    },
    {
      icon: <Battery size={24} />,
      value: "100%",
      label: "系统健康度",
      color: "linear-gradient(135deg, #A1FFCE, #FAFFD1)",
    },
  ];

  // 心情选项
  const moods = [
    { id: "energetic", label: "活力四射", icon: <Zap size={16} /> },
    { id: "calm", label: "平静安宁", icon: <Droplets size={16} /> },
    { id: "creative", label: "创意迸发", icon: <Palette size={16} /> },
    { id: "focused", label: "专注高效", icon: <Brain size={16} /> },
  ];

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  const formatDate = (date: Date) => {
    return date.toLocaleDateString("zh-CN", {
      year: "numeric",
      month: "long",
      day: "numeric",
      weekday: "long",
    });
  };

  return (
    <VibeContainer>
      <Header>
        <div>
          <TitleSection>
            <Sparkles size={32} color="hsl(var(--primary))" />
            <Title>Vibe Zone</Title>
          </TitleSection>
          <Subtitle>
            感受应用的脉搏，调整你的创作氛围。这里是灵感与能量的交汇点。
            当前视觉模式：{activeVisual === "waves" ? "波动" : "粒子"}。
          </Subtitle>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <Wifi size={20} color="hsl(var(--muted-foreground))" />
          <Radio size={20} color="hsl(var(--muted-foreground))" />
          <Volume2 size={20} color="hsl(var(--muted-foreground))" />
        </div>
      </Header>

      {/* 时间显示 */}
      <TimeDisplay>{formatTime(currentTime)}</TimeDisplay>
      <DateDisplay>{formatDate(currentTime)}</DateDisplay>

      {/* 统计数据 */}
      <StatsGrid>
        {stats.map((stat, index) => (
          <StatCard
            key={index}
            onClick={() =>
              setActiveVisual(index % 2 === 0 ? "waves" : "particles")
            }
          >
            <StatIcon $color={stat.color}>{stat.icon}</StatIcon>
            <StatContent>
              <StatValue>{stat.value}</StatValue>
              <StatLabel>{stat.label}</StatLabel>
            </StatContent>
          </StatCard>
        ))}
      </StatsGrid>

      {/* 可视化效果 */}
      <VisualGrid>
        <VisualCard>
          <VisualHeader>
            <Activity size={20} color="hsl(var(--primary))" />
            <VisualTitle>能量波动</VisualTitle>
          </VisualHeader>
          <WaveContainer>
            <Wave $delay={0} />
            <Wave $delay={0.5} />
            <Wave $delay={1} />
          </WaveContainer>
        </VisualCard>

        <VisualCard>
          <VisualHeader>
            <TrendingUp size={20} color="hsl(var(--secondary))" />
            <VisualTitle>活跃度频谱</VisualTitle>
          </VisualHeader>
          <ParticleGrid>
            {particles.map((particle, index) => (
              <Particle
                key={index}
                $height={particle.height}
                $delay={particle.delay}
              />
            ))}
          </ParticleGrid>
        </VisualCard>

        <VisualCard>
          <VisualHeader>
            <Palette size={20} color="hsl(var(--accent))" />
            <VisualTitle>色彩调色板</VisualTitle>
          </VisualHeader>
          <ColorGrid>
            {colors.map((color, index) => (
              <ColorSwatch
                key={index}
                $color={color}
                title={`颜色 ${index + 1}`}
              />
            ))}
          </ColorGrid>
        </VisualCard>

        <VisualCard>
          <VisualHeader>
            <Heart size={20} color="hsl(var(--destructive))" />
            <VisualTitle>心情氛围</VisualTitle>
          </VisualHeader>
          <MoodSection>
            <MoodSelector>
              {moods.map((mood) => (
                <MoodButton
                  key={mood.id}
                  $active={selectedMood === mood.id}
                  onClick={() => setSelectedMood(mood.id)}
                  variant={selectedMood === mood.id ? "default" : "secondary"}
                >
                  {mood.icon}
                  <span style={{ marginLeft: "8px" }}>{mood.label}</span>
                </MoodButton>
              ))}
            </MoodSelector>
            <MoodDisplay $mood={selectedMood}>
              {moods.find((m) => m.id === selectedMood)?.label}
            </MoodDisplay>
          </MoodSection>
        </VisualCard>
      </VisualGrid>

      {/* 操作按钮 */}
      <ActionButtons>
        <Button size="lg" variant="default">
          <Sparkles size={20} style={{ marginRight: "8px" }} />
          刷新氛围
        </Button>
        <Button size="lg" variant="secondary">
          <Music size={20} style={{ marginRight: "8px" }} />
          播放音乐
        </Button>
        <Button size="lg" variant="outline">
          <Sun size={20} style={{ marginRight: "8px" }} />
          切换主题
        </Button>
      </ActionButtons>
    </VibeContainer>
  );
}
