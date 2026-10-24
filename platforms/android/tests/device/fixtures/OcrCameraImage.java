import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.image.BufferedImage;
import java.io.File;
import javax.imageio.ImageIO;

/** 生成模拟器相机的固定合成图；不读取外部图片、字体或用户文件。 */
final class OcrCameraImage {
    public static void main(String[] arguments) throws Exception {
        BufferedImage image = new BufferedImage(1280, 720, BufferedImage.TYPE_INT_RGB);
        Graphics2D canvas = image.createGraphics();
        try {
            canvas.setColor(Color.WHITE);
            canvas.fillRect(0, 0, image.getWidth(), image.getHeight());
            canvas.setColor(Color.BLACK);
            canvas.setFont(new Font(Font.SANS_SERIF, Font.PLAIN, 48));
            canvas.drawString("SYNTHETIC OCR 2468", 220, 360);
        } finally { canvas.dispose(); }
        if (!ImageIO.write(image, "png", new File(arguments[0]))) throw new IllegalStateException("fixture_image");
    }
}
